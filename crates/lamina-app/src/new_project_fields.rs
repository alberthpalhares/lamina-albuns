//! Os campos do Personalizado da etapa 1 (specs NP e ET): medidas, unidade, margens e a faixa de
//! páginas. As decisões ficam no `lamina-editor::new_project` (regra 1).

use lamina_editor::SpecFields;
use lamina_editor::forms::parse_limits;
use lamina_editor::new_project::{SizeField, SizeForm, Unit, is_advanced};
use slint::{ComponentHandle, SharedString};

use crate::NewProjectUi;
use crate::state::App;

const FIELDS: [SizeField; 9] = [
    SizeField::Width,
    SizeField::Height,
    SizeField::Dpi,
    SizeField::BleedOuter,
    SizeField::BleedTopBottom,
    SizeField::BleedSpine,
    SizeField::SafeOuter,
    SizeField::SafeTopBottom,
    SizeField::SafeSpine,
];

pub(crate) fn fill_size(ui: &NewProjectUi, form: &SizeForm) {
    let t = |f: SizeField| SharedString::from(form.text(f));
    ui.set_width(t(SizeField::Width));
    ui.set_height(t(SizeField::Height));
    ui.set_dpi(t(SizeField::Dpi));
    ui.set_bleed_outer(t(SizeField::BleedOuter));
    ui.set_bleed_top(t(SizeField::BleedTopBottom));
    ui.set_bleed_spine(t(SizeField::BleedSpine));
    ui.set_safe_outer(t(SizeField::SafeOuter));
    ui.set_safe_top(t(SizeField::SafeTopBottom));
    ui.set_safe_spine(t(SizeField::SafeSpine));
    ui.set_starts(form.fields.starts_single);
    ui.set_ends(form.fields.ends_single);
}

fn typed(ui: &NewProjectUi, field: SizeField) -> SharedString {
    match field {
        SizeField::Width => ui.get_width(),
        SizeField::Height => ui.get_height(),
        SizeField::Dpi => ui.get_dpi(),
        SizeField::BleedOuter => ui.get_bleed_outer(),
        SizeField::BleedTopBottom => ui.get_bleed_top(),
        SizeField::BleedSpine => ui.get_bleed_spine(),
        SizeField::SafeOuter => ui.get_safe_outer(),
        SizeField::SafeTopBottom => ui.get_safe_top(),
        SizeField::SafeSpine => ui.get_safe_spine(),
    }
}

/// Um campo mudou: cada texto diferente do que o formulário mostra é lido de novo. Um erro num
/// campo dos ajustes da gráfica abre o bloco recolhido (ET, regra 6).
fn size_changed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let Some(wizard) = app.wizard.as_mut() else {
        return;
    };
    let mut wrong = None;
    for field in FIELDS {
        let text = typed(&ui, field);
        if text.as_str() != wizard.size.text(field)
            && let Err(e) = wizard.size.set(field, &text)
        {
            wrong = wrong.or(Some((field, e)));
        }
    }
    wizard.size.fields.starts_single = ui.get_starts();
    wizard.size.fields.ends_single = ui.get_ends();
    crate::new_project::refresh(app);
    if let Some((field, e)) = wrong {
        ui.set_error(e.into());
        if is_advanced(field) {
            ui.set_advanced_open(true);
        }
    }
}

/// As medidas digitadas, para "Salvar como preset…" (spec PR).
pub fn fields(app: &App) -> Result<SpecFields, String> {
    let wizard = app.wizard.as_ref().ok_or_else(String::new)?;
    wizard.size.spec().map_err(str::to_owned)?;
    Ok(wizard.size.fields.clone())
}

/// A faixa de páginas digitada na etapa 1.
pub fn limits(app: &App) -> Result<lamina_core::PageLimits, String> {
    let w = app.window.upgrade().ok_or_else(String::new)?;
    let ui = w.global::<NewProjectUi>();
    parse_limits(&ui.get_min_pages(), &ui.get_max_pages())
}

fn unit_changed(app: &mut App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let Some(wizard) = app.wizard.as_mut() else {
        return;
    };
    let unit = usize::try_from(index)
        .ok()
        .and_then(|i| Unit::ALL.get(i).copied())
        .unwrap_or_default();
    wizard.size.set_unit(unit);
    wizard.size.open_spread = ui.get_open_spread();
    fill_size(&ui, &wizard.size);
    app.editor.remember_unit(unit);
    crate::new_project::refresh(app);
}

fn same_margins(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    if let Some(wizard) = app.wizard.as_mut() {
        wizard.size.same_margins();
        fill_size(&w.global::<NewProjectUi>(), &wizard.size);
    }
    crate::new_project::refresh(app);
}

pub fn connect(ui: &NewProjectUi, app: &crate::new_project::Shared) {
    let with = |f: fn(&mut App, i32)| {
        let a = app.clone();
        move |i| f(&mut a.borrow_mut(), i)
    };
    let on = |f: fn(&mut App)| {
        let a = app.clone();
        move || f(&mut a.borrow_mut())
    };
    ui.on_unit_changed(with(unit_changed));
    ui.on_size_changed(on(size_changed));
    ui.on_same_margins(on(same_margins));
}
