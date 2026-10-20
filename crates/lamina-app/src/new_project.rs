//! O Novo projeto em três etapas (spec NP), etapas 1 e 2: copia o `lamina-editor::new_project`
//! para a tela e traduz o que o fotógrafo digita. A etapa 3 e o "Criar" ficam em
//! `new_project_file.rs`. Nenhuma decisão aqui (regra 1).

use std::cell::RefCell;
use std::rc::Rc;

use lamina_core::{AlbumLook, Border, CropLevel, Mm, Mounting};
use lamina_editor::forms::{limits_text, parse_limits};
use lamina_editor::new_project::{
    Format, NewProject, SizeField, SizeForm, Unit, drawing, formats, mount_preview,
};
use lamina_editor::{Preset, SpecFields, color_hex, parse_color};
use lamina_layout::Reuse;
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer, SharedString};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, DrawData, FormatCard, NewProjectUi};

type Shared = Rc<RefCell<App>>;

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

/// Abre a etapa 1 com os padrões (o cartão do formato do tamanho padrão).
pub fn start(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let wizard = NewProject::new(SizeForm::default());
    let ui = w.global::<NewProjectUi>();
    ui.set_tab(0);
    ui.set_unit(unit_index(wizard.size.unit));
    ui.set_open_spread(false);
    ui.set_min_pages("".into());
    ui.set_max_pages("".into());
    app.new_export = lamina_core::ExportPrefs::default();
    fill_size(&ui, &wizard.size);
    fill_mount(&ui, &wizard);
    app.wizard = Some(wizard);
    crate::new_project_file::reset(app);
    pick_format(app, format_index(Format::of(&SpecFields::default())));
    refresh(app);
    ui.set_show(true);
}

fn ui_of(app: &App) -> Option<(AppWindow, &NewProject)> {
    Some((app.window.upgrade()?, app.wizard.as_ref()?))
}

/// O que muda com as medidas: rótulo, resumo, erro, desenho e a prévia da etapa 2.
pub fn refresh(app: &App) {
    let Some((w, wizard)) = ui_of(app) else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    ui.set_step(i32::from(wizard.step));
    ui.set_label(wizard.size.label().into());
    ui.set_summary(wizard.size.summary().unwrap_or_default().into());
    let limits = parse_limits(&ui.get_min_pages(), &ui.get_max_pages()).err();
    let error = wizard.size.error().map(str::to_owned).or(limits);
    ui.set_error(error.unwrap_or_default().into());
    if let Ok(spec) = wizard.size.spec() {
        ui.set_drawing(draw_data(&spec));
        let previews = mount_preview(&spec, &wizard.style(), wizard.lay_flat);
        ui.set_previews(model(previews.iter().map(picture).collect()));
    }
}

// Frações e proporções da tela cabem com folga em f32.
#[allow(clippy::cast_possible_truncation)]
fn f(v: f64) -> f32 {
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

fn fill_size(ui: &NewProjectUi, form: &SizeForm) {
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

/// Um campo mudou: cada texto diferente do que o formulário mostra é lido de novo.
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
            wrong = wrong.or(Some(e));
        }
    }
    wizard.size.fields.starts_single = ui.get_starts();
    wizard.size.fields.ends_single = ui.get_ends();
    refresh(app);
    if let Some(e) = wrong {
        ui.set_error(e.into());
    }
}

fn unit_index(unit: Unit) -> i32 {
    Unit::ALL
        .iter()
        .position(|u| *u == unit)
        .map_or(1, |i| i32::try_from(i).unwrap_or(1))
}

fn format_index(format: Format) -> i32 {
    Format::ALL
        .iter()
        .position(|f| *f == format)
        .map_or(0, |i| i32::try_from(i).unwrap_or(0))
}

/// Os cartões dos três formatos e os tamanhos do escolhido (regra 1).
fn pick_format(app: &mut App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let groups = formats(&app.presets.all());
    let cards = groups
        .iter()
        .map(|(format, sizes)| FormatCard {
            label: format.label().into(),
            aspect: sizes
                .first()
                .map_or(1.0, |p| f(p.fields.width_cm / p.fields.height_cm)),
        })
        .collect();
    ui.set_formats(model(cards));
    ui.set_format(index);
    let sizes = usize::try_from(index)
        .ok()
        .and_then(|i| groups.get(i))
        .map(|(_, s)| {
            s.iter()
                .map(|p| SharedString::from(p.name.as_str()))
                .collect()
        })
        .unwrap_or_default();
    ui.set_sizes(model(sizes));
}

fn pick_size(app: &mut App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let groups = formats(&app.presets.all());
    let chosen = usize::try_from(ui.get_format())
        .ok()
        .and_then(|f| groups.get(f))
        .and_then(|(_, s)| usize::try_from(index).ok().and_then(|i| s.get(i)))
        .cloned();
    if let Some(preset) = chosen {
        choose_preset(app, &preset);
    }
}

/// Um tamanho escolhido (cartão ou lista de presets) preenche o Personalizado (regra 2).
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
    refresh(app);
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
        .and_then(|i| Unit::ALL.get(i).copied());
    wizard.size.set_unit(unit.unwrap_or_default());
    wizard.size.open_spread = ui.get_open_spread();
    fill_size(&ui, &wizard.size);
}

fn same_margins(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    if let Some(wizard) = app.wizard.as_mut() {
        wizard.size.same_margins();
        fill_size(&w.global::<NewProjectUi>(), &wizard.size);
    }
    refresh(app);
}

fn fill_mount(ui: &NewProjectUi, wizard: &NewProject) {
    let m = wizard.mounting;
    ui.set_lay_flat(wizard.lay_flat);
    ui.set_crop(match m.crop {
        CropLevel::Whole => 0,
        CropLevel::Light => 1,
        CropLevel::Free => 2,
    });
    ui.set_bleed_page(m.bleed_page);
    ui.set_mosaics(m.mosaics);
    ui.set_reuse(1);
    ui.set_background(color_hex(wizard.look.background).into());
    ui.set_gap(Unit::Mm.show(wizard.gap.get()).into());
    ui.set_border(Unit::Mm.show(wizard.look.border.width.get()).into());
    ui.set_border_color(color_hex(wizard.look.border.color).into());
    ui.set_radius(Unit::Mm.show(wizard.look.radius.get()).into());
    ui.set_look_error("".into());
}

/// A etapa 2 mudou: montagem e aparência no editor e a prévia de novo (regra 7).
fn mount_changed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let Some(wizard) = app.wizard.as_mut() else {
        return;
    };
    wizard.lay_flat = ui.get_lay_flat();
    wizard.mounting = Mounting {
        crop: match ui.get_crop() {
            0 => CropLevel::Whole,
            1 => CropLevel::Light,
            _ => CropLevel::Free,
        },
        bleed_page: ui.get_bleed_page(),
        mosaics: ui.get_mosaics(),
    };
    wizard.reuse = match ui.get_reuse() {
        0 => Reuse::Low,
        2 => Reuse::High,
        _ => Reuse::Medium,
    };
    let look = read_look(&ui);
    ui.set_look_error(look.as_ref().err().copied().unwrap_or_default().into());
    if let Ok((look, gap)) = look {
        wizard.look = look;
        wizard.gap = gap;
    }
    refresh(app);
}

fn read_look(ui: &NewProjectUi) -> Result<(AlbumLook, Mm), &'static str> {
    const COLOR: &str = "Use uma cor como #FFFFFF.";
    let mm = |text: SharedString| Unit::Mm.read(&text).map(Mm);
    let look = AlbumLook {
        background: parse_color(&ui.get_background()).ok_or(COLOR)?,
        border: Border {
            width: mm(ui.get_border())?,
            color: parse_color(&ui.get_border_color()).ok_or(COLOR)?,
        },
        radius: mm(ui.get_radius())?,
    };
    Ok((look, mm(ui.get_gap())?))
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
    let with = |f: fn(&mut App, i32)| {
        let a = app.clone();
        move |i| f(&mut a.borrow_mut(), i)
    };
    ui.on_pick_format(with(pick_format));
    ui.on_pick_size(with(pick_size));
    ui.on_unit_changed(with(unit_changed));
    ui.on_size_changed(on(size_changed));
    ui.on_same_margins(on(same_margins));
    ui.on_mount_changed(on(mount_changed));
    ui.on_next(on(|app| step(app, true)));
    ui.on_back(on(|app| step(app, false)));
    ui.on_cancel(on(cancel));
    crate::new_project_file::connect(window, app);
}
