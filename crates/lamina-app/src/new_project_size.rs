//! A etapa 1 do Novo projeto (specs NP e ET): o filtro por laboratório, os cartões de formato e o
//! seletor de tamanho. Os campos do Personalizado ficam em `new_project_fields.rs`. As decisões
//! ficam no `lamina-editor::new_project` (regra 1).

use lamina_editor::Preset;
use lamina_editor::forms::limits_text;
use lamina_editor::new_project::{
    ADD_LAB, ALL_LABS, Format, Unit, labs, size_choices, size_labels,
};
use slint::{ComponentHandle, SharedString};

use crate::convert::model;
use crate::new_project_fields::fill_size;
use crate::state::App;
use crate::{FormatCard, NewProjectUi};

pub(crate) fn unit_index(unit: Unit) -> i32 {
    Unit::ALL
        .iter()
        .position(|u| *u == unit)
        .map_or(1, |i| i32::try_from(i).unwrap_or(1))
}

pub(crate) fn format_index(format: Format) -> i32 {
    Format::ALL
        .iter()
        .position(|f| *f == format)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0))
}

fn index(i: Option<usize>) -> i32 {
    i.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1)
}

/// O laboratório do filtro (0 = "Todos"; o último item é o convite).
fn lab_filter(app: &App, ui: &NewProjectUi) -> Option<String> {
    let found = labs(&app.presets.all());
    usize::try_from(ui.get_lab() - 1)
        .ok()
        .and_then(|i| found.get(i).cloned())
}

/// Os tamanhos de cada formato, com o filtro de laboratório (ET, regra 4).
fn groups(app: &App, ui: &NewProjectUi) -> Vec<(Format, Vec<Preset>)> {
    size_choices(&app.presets.all(), lab_filter(app, ui).as_deref())
}

/// Mostra o filtro, os cartões e os tamanhos do formato `index`, com o tamanho escolhido marcado
/// (ET, regras 1, 2 e 4). Não muda as medidas.
pub(crate) fn show_format(app: &App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let found = labs(&app.presets.all());
    if found.is_empty() {
        ui.set_labs(model(Vec::new()));
    } else {
        let mut names: Vec<SharedString> = vec![ALL_LABS.into()];
        names.extend(found.iter().map(|l| SharedString::from(l.as_str())));
        names.push(ADD_LAB.into());
        ui.set_labs(model(names));
    }
    let groups = groups(app, &ui);
    let cards = groups
        .iter()
        .map(|(format, sizes)| FormatCard {
            label: format.label().into(),
            aspect: aspect(*format, sizes),
            enabled: !sizes.is_empty(),
        })
        .collect();
    ui.set_formats(model(cards));
    ui.set_format(index);
    let all_labs = lab_filter(app, &ui).is_none();
    let (labels, chosen) = usize::try_from(index)
        .ok()
        .and_then(|i| groups.get(i))
        .map(|(format, sizes)| {
            let labels = size_labels(*format, sizes, all_labs);
            (labels, chosen_index(app, sizes))
        })
        .unwrap_or_default();
    ui.set_sizes(model(labels.into_iter().map(SharedString::from).collect()));
    ui.set_size(chosen);
}

fn aspect(format: Format, sizes: &[Preset]) -> f32 {
    let fallback = match format {
        Format::Square => 1.0,
        Format::Landscape => 1.4,
        Format::Portrait => 0.7,
    };
    sizes.first().map_or(fallback, |p| {
        crate::new_project::f(p.fields.width_cm / p.fields.height_cm)
    })
}

/// A posição do tamanho escolhido na lista; −1 com medidas personalizadas.
fn chosen_index(app: &App, sizes: &[Preset]) -> i32 {
    let chosen = app.wizard.as_ref().and_then(|w| w.size.chosen());
    index(
        chosen.and_then(|(name, lab)| sizes.iter().position(|p| p.name == *name && p.lab == *lab)),
    )
}

/// Clique num cartão: o formato e o primeiro tamanho dele, se o escolhido não é desse formato.
fn pick_format(app: &mut App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let sizes = usize::try_from(index)
        .ok()
        .and_then(|i| groups(app, &ui).into_iter().nth(i))
        .map(|(_, s)| s)
        .unwrap_or_default();
    match sizes.first() {
        Some(first) if chosen_index(app, &sizes) < 0 => choose_preset(app, first),
        Some(_) => {}
        None => return,
    }
    show_format(app, index);
}

fn pick_size(app: &mut App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let chosen = usize::try_from(ui.get_format())
        .ok()
        .and_then(|f| groups(app, &ui).into_iter().nth(f))
        .and_then(|(_, s)| {
            usize::try_from(index)
                .ok()
                .and_then(|i| s.into_iter().nth(i))
        });
    if let Some(preset) = chosen {
        choose_preset(app, &preset);
    }
    show_format(app, ui.get_format());
}

/// O filtro de laboratório mudou. O convite do fim abre o Personalizado com os ajustes da gráfica,
/// onde fica "Salvar como preset…" (ET, regra 5).
fn pick_lab(app: &mut App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    if index + 1 >= index_of(slint::Model::row_count(&ui.get_labs())) {
        ui.set_lab(0);
        ui.set_tab(1);
        ui.set_advanced_open(true);
        return show_format(app, ui.get_format());
    }
    let groups = groups(app, &ui);
    let current = usize::try_from(ui.get_format()).unwrap_or(0);
    let format = if groups.get(current).is_some_and(|(_, s)| !s.is_empty()) {
        current
    } else {
        groups.iter().position(|(_, s)| !s.is_empty()).unwrap_or(0)
    };
    pick_format(app, index_of(format));
}

fn index_of(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(0)
}

/// Um tamanho escolhido (seletor ou lista de presets) preenche o Personalizado (NP, regra 2).
pub fn choose_preset(app: &mut App, preset: &Preset) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let Some(wizard) = app.wizard.as_mut() else {
        return;
    };
    wizard.size.choose(preset);
    fill_size(&ui, &wizard.size);
    let (min, max) = limits_text(preset.limits);
    ui.set_min_pages(min.into());
    ui.set_max_pages(max.into());
    app.new_export = preset.export;
    crate::new_project::refresh(app);
}

pub fn connect(ui: &NewProjectUi, app: &crate::new_project::Shared) {
    let with = |f: fn(&mut App, i32)| {
        let a = app.clone();
        move |i| f(&mut a.borrow_mut(), i)
    };
    ui.on_pick_lab(with(pick_lab));
    ui.on_pick_format(with(pick_format));
    ui.on_pick_size(with(pick_size));
}
