//! O Novo projeto em três etapas (spec NP): copia o `lamina-editor::new_project` para a tela. A
//! etapa 1 fica em `new_project_size.rs`, a 2 em `new_project_mount.rs` e a 3 e o "Criar" em
//! `new_project_file.rs`. Nenhuma decisão aqui (regra 1).

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::forms::{limits_text, parse_limits};
use lamina_editor::new_project::{
    Format, NewProject, SizeForm, advanced_error, drawing, last_preset, matching_preset,
    mount_preview, short_summary,
};
use lamina_editor::{Human, PALETTE, SpecFields, color_hex};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use crate::convert::model;
use crate::new_project_fields::fill_size;
use crate::new_project_size::{format_index, show_format, unit_index};
use crate::state::App;
use crate::{AppWindow, ColorChoice, DrawData, NewProjectUi};

pub use crate::new_project_fields::{fields, limits};
pub use crate::new_project_size::choose_preset;

pub(crate) type Shared = Rc<RefCell<App>>;

/// Abre a etapa 1 com os padrões, na unidade de sempre e com o tamanho do último álbum criado
/// (NP regras 1 e 3, ET regra 1).
pub fn start(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let strips = app.editor.strips();
    let mut size = SizeForm::default();
    size.set_unit(strips.unit);
    let wizard = NewProject::new(size);
    let ui = w.global::<NewProjectUi>();
    ui.set_mode(0);
    ui.set_tab(0);
    ui.set_binder(lamina_editor::new_project::ALL_BINDERS.into());
    ui.set_binder_search("".into());
    ui.set_advanced_open(false);
    ui.set_unit(unit_index(wizard.size.unit));
    ui.set_open_spread(false);
    ui.set_min_pages("".into());
    ui.set_max_pages("".into());
    app.new_export = lamina_core::ExportPrefs::default();
    fill_size(&ui, &wizard.size);
    let palette = PALETTE
        .iter()
        .map(|(_, c)| ColorChoice {
            c: slint::Color::from_rgb_u8(c.r, c.g, c.b),
            hex: color_hex(*c).into(),
        })
        .collect();
    ui.set_palette(model(palette));
    crate::new_project_mount::fill_mount(&ui, &wizard);
    app.wizard = Some(wizard);
    crate::new_project_file::reset(app);
    let presets = app.presets.all();
    // O tamanho do último álbum; sem ele, o primeiro do último formato.
    let chosen = last_preset(&presets, strips.last_size.as_ref()).or_else(|| {
        presets
            .iter()
            .find(|p| Format::of(&p.fields) == strips.format)
            .cloned()
    });
    let format = chosen
        .as_ref()
        .map_or(strips.format, |p| Format::of(&p.fields));
    if let Some(preset) = &chosen {
        choose_preset(app, preset);
    }
    show_format(app, format_index(format));
    refresh(app);
    ui.set_show(true);
}

/// Sem álbum aberto, arquivos soltos na janela abrem o Novo projeto com eles já escolhidos na
/// etapa 3 (spec IM, regra 6).
pub fn start_with_files(app: &mut App, files: Vec<std::path::PathBuf>) {
    let photos = lamina_editor::importing::photos_in(&files);
    if photos == 0 {
        app.notice = lamina_editor::importing::NO_PHOTOS_DROPPED.to_owned();
        return app.refresh_current();
    }
    start(app);
    crate::new_project_file::use_files(app, files, photos);
}

/// Arquivo → Especificação do álbum…: a etapa 1 com os valores do álbum aberto (NP regra 11). O
/// tamanho pronto igual ao do álbum vem marcado (ET, regra 10).
pub fn open_spec(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.editor.session()) else {
        return;
    };
    let project = session.project();
    let fields = SpecFields::from_spec(&project.spec);
    let mut size = SizeForm::new(fields.clone());
    size.set_unit(app.editor.strips().unit);
    let same = matching_preset(&app.presets.all(), &fields);
    if let Some(preset) = &same {
        size.choose(preset);
    }
    let (min, max) = limits_text(project.page_limits);
    app.new_export = session.export_prefs();
    let wizard = NewProject::new(size);
    let ui = w.global::<NewProjectUi>();
    ui.set_mode(1);
    ui.set_tab(if same.is_some() { 0 } else { 1 });
    ui.set_binder(lamina_editor::new_project::ALL_BINDERS.into());
    ui.set_binder_search("".into());
    ui.set_advanced_open(false);
    ui.set_unit(unit_index(wizard.size.unit));
    ui.set_open_spread(false);
    ui.set_min_pages(min.into());
    ui.set_max_pages(max.into());
    fill_size(&ui, &wizard.size);
    app.wizard = Some(wizard);
    show_format(app, format_index(Format::of(&fields)));
    refresh(app);
    ui.set_show(true);
}

/// Arquivo → Montagem do álbum…: a etapa 2 com a montagem do álbum aberto (regra 11).
pub fn open_mount(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.editor.session()) else {
        return;
    };
    let project = session.project();
    let mut wizard = NewProject::new(SizeForm::new(SpecFields::from_spec(&project.spec)));
    wizard.step = 2;
    wizard.mounting = project.mounting;
    wizard.lay_flat = project.spec.mode() == lamina_core::DesignMode::Spread;
    let ui = w.global::<NewProjectUi>();
    ui.set_mode(2);
    crate::new_project_mount::fill_mount(&ui, &wizard);
    app.wizard = Some(wizard);
    refresh(app);
    ui.set_show(true);
}

/// "Aplicar" nos dois modos do álbum aberto: a especificação (uma edição que refaz as lâminas
/// com o mesmo desenho; spec PR) ou a montagem (sem refazer as lâminas).
fn apply(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let Some(wizard) = app.wizard.take() else {
        return;
    };
    ui.set_show(false);
    let done = if ui.get_mode() == 1 {
        apply_spec(app, &wizard)
    } else {
        app.editor
            .set_mounting(wizard.mounting, wizard.lay_flat)
            .map(|()| None)
    };
    match done {
        Ok(notice) => {
            app.notice = notice.unwrap_or_default();
            app.project_changed();
        }
        Err(e) => crate::files::show_error(app, &e.human()),
    }
    w.invoke_focus_main();
}

fn apply_spec(
    app: &mut App,
    wizard: &NewProject,
) -> Result<Option<String>, lamina_editor::EditorError> {
    let mode = app
        .editor
        .session()
        .map(|s| s.project().spec.mode())
        .unwrap_or_default();
    let (Ok(spec), Ok(limits)) = (wizard.size.spec(), limits(app)) else {
        return Ok(None);
    };
    let notice = app.editor.respec(spec.with_mode(mode), limits)?;
    if let Some(session) = app.editor.session_mut() {
        session.set_export_prefs(app.new_export);
    }
    Ok(notice)
}

fn ui_of(app: &App) -> Option<(AppWindow, &NewProject)> {
    Some((app.window.upgrade()?, app.wizard.as_ref()?))
}

/// O que muda com as medidas: rótulo, resumos, erro, desenho e a prévia da etapa 2. Um erro nos
/// ajustes da gráfica abre o bloco recolhido (ET, regra 6).
pub fn refresh(app: &App) {
    let Some((w, wizard)) = ui_of(app) else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    ui.set_step(i32::from(wizard.step));
    ui.set_label(wizard.size.label().into());
    ui.set_summary(wizard.size.summary().unwrap_or_default().into());
    let limits = parse_limits(&ui.get_min_pages(), &ui.get_max_pages());
    let short = short_summary(&wizard.size, limits.clone().unwrap_or_default());
    ui.set_short_summary(short.unwrap_or_default().into());
    if advanced_error(&wizard.size, limits.is_err()) {
        ui.set_advanced_open(true);
    }
    let error = wizard.size.error().map(str::to_owned).or(limits.err());
    ui.set_error(error.unwrap_or_default().into());
    if let Ok(spec) = wizard.size.spec() {
        ui.set_drawing(draw_data(&spec));
        let previews = mount_preview(&spec, &wizard.style(), wizard.lay_flat);
        ui.set_previews(model(previews.iter().map(picture).collect()));
    }
}

// Frações e proporções da tela cabem com folga em f32.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn f(v: f64) -> f32 {
    v as f32
}

fn draw_data(spec: &lamina_core::AlbumSpec) -> DrawData {
    let d = drawing(spec);
    DrawData {
        aspect: f(d.aspect),
        tx: f(d.trim[0]),
        ty: f(d.trim[1]),
        tw: f(d.trim[2]),
        th: f(d.trim[3]),
        lx: f(d.safe_left[0]),
        ly: f(d.safe_left[1]),
        lw: f(d.safe_left[2]),
        lh: f(d.safe_left[3]),
        rx: f(d.safe_right[0]),
        ry: f(d.safe_right[1]),
        rw: f(d.safe_right[2]),
        rh: f(d.safe_right[3]),
        starts: d.starts_single,
        ends: d.ends_single,
    }
}

fn picture(rgba: &lamina_editor::screens::Rgba) -> Image {
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
    );
    Image::from_rgba8(buffer)
}

fn step(app: &mut App, forward: bool) {
    if let Some(wizard) = app.wizard.as_mut() {
        if forward {
            wizard.advance();
        } else {
            wizard.back();
        }
    }
    crate::new_project_file::refresh(app);
    refresh(app);
}

fn cancel(app: &mut App) {
    app.wizard = None;
    if let Some(w) = app.window.upgrade() {
        w.global::<NewProjectUi>().set_show(false);
        w.invoke_focus_main();
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<NewProjectUi>();
    let on = |f: fn(&mut App)| {
        let a = app.clone();
        move || f(&mut a.borrow_mut())
    };
    crate::new_project_size::connect(&ui, app);
    crate::new_project_fields::connect(&ui, app);
    crate::new_project_mount::connect(&ui, app);
    ui.on_next(on(|app| step(app, true)));
    ui.on_back(on(|app| step(app, false)));
    ui.on_cancel(on(cancel));
    ui.on_apply(on(apply));
    crate::new_project_file::connect(window, app);
}
