//! A "Especificação do álbum" e a lista "Meus presets" (spec PR): aplicar uma especificação nova
//! ao álbum aberto, e renomear ou apagar os presets do fotógrafo.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::SpecFields;
use lamina_editor::forms::{limits_text, parse_limits};
use slint::ComponentHandle;

use crate::presets::{custom_index, fill_spec, refresh, spec_text};
use crate::state::App;
use crate::{AppWindow, PresetUi};

type Shared = Rc<RefCell<App>>;

/// Arquivo → Especificação do álbum…: os campos do álbum aberto.
pub fn open_spec(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.editor.session()) else {
        return;
    };
    let project = session.project();
    refresh(app);
    let ui = w.global::<PresetUi>();
    fill_spec(&ui, &SpecFields::from_spec(&project.spec));
    let (min, max) = limits_text(project.page_limits);
    ui.set_spec_min(min.into());
    ui.set_spec_max(max.into());
    ui.set_spec_preset(custom_index(app));
    app.spec_export = session.export_prefs();
    spec_changed(app);
    ui.set_show_spec(true);
}

pub(crate) fn spec_changed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PresetUi>();
    let check = spec_text(&ui)
        .parse()
        .and_then(|f| f.to_spec().map_err(|e| e.to_string()))
        .and_then(|_| parse_limits(&ui.get_spec_min(), &ui.get_spec_max()).map(|_| ()));
    ui.set_spec_error(check.err().unwrap_or_default().into());
}

/// "Aplicar": uma edição só, que Ctrl+Z desfaz (regra 6).
pub(crate) fn spec_apply(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PresetUi>();
    let parsed = spec_text(&ui)
        .parse()
        .and_then(|f| f.to_spec().map_err(|e| e.to_string()))
        .and_then(|spec| parse_limits(&ui.get_spec_min(), &ui.get_spec_max()).map(|l| (spec, l)));
    let (spec, limits) = match parsed {
        Ok(v) => v,
        Err(e) => return ui.set_spec_error(e.into()),
    };
    match app.editor.respec(spec, limits) {
        Ok(notice) => {
            if let Some(session) = app.editor.session_mut() {
                session.set_export_prefs(app.spec_export);
            }
            ui.set_show_spec(false);
            app.notice = notice.unwrap_or_default();
            app.project_changed();
            w.invoke_focus_main();
        }
        Err(e) => ui.set_spec_error(e.to_string().into()),
    }
}

/// Arquivo → Presets…
pub fn open_list(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    refresh(app);
    let ui = w.global::<PresetUi>();
    ui.set_list_selected(-1);
    ui.set_list_name("".into());
    ui.set_list_message("".into());
    ui.set_show_list(true);
}

pub(crate) fn selected(app: &App, ui: &PresetUi<'_>) -> Option<String> {
    let i = usize::try_from(ui.get_list_selected()).ok()?;
    app.presets.user().get(i).map(|p| p.name.clone())
}

pub(crate) fn list_rename(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PresetUi>();
    let Some(old) = selected(app, &ui) else {
        return;
    };
    match app.presets.rename(&old, &ui.get_list_name()) {
        Ok(_) => {
            ui.set_list_message("".into());
            ui.set_list_selected(-1);
            refresh(app);
        }
        Err(e) => ui.set_list_message(e.to_string().into()),
    }
}

pub(crate) fn list_delete(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PresetUi>();
    let Some(name) = selected(app, &ui) else {
        return;
    };
    match app.presets.remove(&name) {
        Ok(_) => {
            ui.set_list_message("".into());
            ui.set_list_selected(-1);
            ui.set_list_name("".into());
            refresh(app);
        }
        Err(e) => ui.set_list_message(e.to_string().into()),
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PresetUi>();
    let a = app.clone();
    ui.on_spec_changed(move || spec_changed(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_spec_apply(move || spec_apply(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_list_rename(move || list_rename(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_list_delete(move || list_delete(&mut a.borrow_mut()));
}
