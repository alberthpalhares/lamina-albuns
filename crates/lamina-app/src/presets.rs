//! Presets e especificação do álbum na janela (spec PR): escolher um preset no "Novo álbum" ou na
//! "Especificação do álbum", salvar o atual como preset, a lista dos presets do fotógrafo e
//! aplicar uma especificação nova. As decisões ficam no `lamina-editor`.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::forms::{SpecText, limits_text, parse_limits};
use lamina_editor::{Preset, SpecFields};
use slint::{ComponentHandle, SharedString};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, PresetUi};

type Shared = Rc<RefCell<App>>;

/// Destino de um preset escolhido ou salvo: 0 = o "Novo álbum"; 1 = a "Especificação do álbum".
const NEW: i32 = 0;

use lamina_editor::naming::{CUSTOM_PRESET as CUSTOM, Named};

/// Os nomes na lista (os do Lâmina, os do fotógrafo e "Personalizado") e a lista dos do fotógrafo.
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PresetUi>();
    let mut names: Vec<SharedString> = app
        .presets
        .all()
        .into_iter()
        .map(|p| p.name.into())
        .collect();
    names.push(CUSTOM.into());
    ui.set_presets(model(names));
    let user: Vec<SharedString> = app
        .presets
        .user()
        .iter()
        .map(|p| p.name.clone().into())
        .collect();
    ui.set_user_presets(model(user));
}

/// Posição de "Personalizado" na lista.
pub fn custom_index(app: &App) -> i32 {
    i32::try_from(app.presets.all().len()).unwrap_or(0)
}

fn index_of(app: &App, name: &str) -> i32 {
    app.presets
        .all()
        .iter()
        .position(|p| p.name == name)
        .and_then(|i| i32::try_from(i).ok())
        .unwrap_or_else(|| custom_index(app))
}

pub(crate) fn spec_text(ui: &PresetUi<'_>) -> SpecText {
    SpecText {
        width_cm: ui.get_spec_width().into(),
        height_cm: ui.get_spec_height().into(),
        dpi: ui.get_spec_dpi().into(),
        bleed_mm: ui.get_spec_bleed().into(),
        safe_mm: ui.get_spec_safe().into(),
        spine_mm: ui.get_spec_spine().into(),
        starts_single: ui.get_spec_starts(),
        ends_single: ui.get_spec_ends(),
    }
}

pub(crate) fn fill_spec(ui: &PresetUi<'_>, fields: &SpecFields) {
    let t = SpecText::from(fields);
    ui.set_spec_width(t.width_cm.into());
    ui.set_spec_height(t.height_cm.into());
    ui.set_spec_dpi(t.dpi.into());
    ui.set_spec_bleed(t.bleed_mm.into());
    ui.set_spec_safe(t.safe_mm.into());
    ui.set_spec_spine(t.spine_mm.into());
    ui.set_spec_starts(t.starts_single);
    ui.set_spec_ends(t.ends_single);
}

/// Um preset escolhido na lista: preenche os campos do destino (que continuam editáveis).
fn picked(app: &mut App, target: i32, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let Some(preset) = usize::try_from(index)
        .ok()
        .and_then(|i| app.presets.all().into_iter().nth(i))
    else {
        return;
    };
    let ui = w.global::<PresetUi>();
    let (min, max) = limits_text(preset.limits);
    if target == NEW {
        crate::new_album::fill(&w, &preset.fields);
        ui.set_new_min(min.into());
        ui.set_new_max(max.into());
        app.new_export = preset.export;
        crate::new_album::changed(app);
        crate::new_album::suggest(app);
    } else {
        fill_spec(&ui, &preset.fields);
        ui.set_spec_min(min.into());
        ui.set_spec_max(max.into());
        app.spec_export = preset.export;
        crate::spec_dialog::spec_changed(app);
    }
}

/// "Salvar como preset…": pede o nome.
fn save_as(app: &mut App, target: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    app.preset_target = target;
    app.name_guard.reset();
    let ui = w.global::<PresetUi>();
    ui.set_name("".into());
    ui.set_name_message("".into());
    ui.set_show_name(true);
}

/// O preset com os campos do destino, ou a mensagem do campo errado.
fn current(app: &App, w: &AppWindow, name: &str) -> Result<Preset, String> {
    let ui = w.global::<PresetUi>();
    let (fields, limits, export) = if app.preset_target == NEW {
        let fields = crate::new_album::fields(w)?;
        (
            fields,
            parse_limits(&ui.get_new_min(), &ui.get_new_max())?,
            app.new_export,
        )
    } else {
        let fields = spec_text(&ui).parse()?;
        (
            fields,
            parse_limits(&ui.get_spec_min(), &ui.get_spec_max())?,
            app.spec_export,
        )
    };
    Ok(Preset {
        name: name.to_owned(),
        fields,
        limits,
        export,
    })
}

/// "Salvar": nome repetido pede um segundo clique para substituir (regra 2).
fn name_save(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PresetUi>();
    let name = ui.get_name().trim().to_owned();
    let preset = match current(app, &w, &name) {
        Ok(p) => p,
        Err(e) => return ui.set_name_message(e.into()),
    };
    let exists = app.presets.exists(&name);
    if let Err(text) = app.name_guard.check(Named::Preset, &name, exists) {
        return ui.set_name_message(text.into());
    }
    if let Err(e) = app.presets.save(preset) {
        return ui.set_name_message(e.to_string().into());
    }
    ui.set_show_name(false);
    refresh(app);
    let index = index_of(app, &name);
    if app.preset_target == NEW {
        ui.set_new_preset(index);
    } else {
        ui.set_spec_preset(index);
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PresetUi>();
    let a = app.clone();
    ui.on_picked(move |target, i| picked(&mut a.borrow_mut(), target, i));
    let a = app.clone();
    ui.on_save_as(move |target| save_as(&mut a.borrow_mut(), target));
    let a = app.clone();
    ui.on_name_save(move || name_save(&mut a.borrow_mut()));
    crate::spec_dialog::connect(window, app);
    let w = window.as_weak();
    ui.on_closed(move || {
        if let Some(w) = w.upgrade() {
            w.invoke_focus_main();
        }
    });
}
