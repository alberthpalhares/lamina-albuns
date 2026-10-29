//! A etapa 2 do Novo projeto (spec NP): montagem, aparência, o quadro de cores e a prévia. As
//! decisões ficam no `lamina-editor::new_project` (regra 1).

use lamina_core::{AlbumLook, Border, CropLevel, Mm, Mounting};
use lamina_editor::new_project::{NewProject, Unit, picker_color};
use lamina_editor::{color_hex, parse_color};
use lamina_layout::Reuse;
use slint::{ComponentHandle, SharedString};

use crate::NewProjectUi;
use crate::state::App;
pub(crate) fn fill_mount(ui: &NewProjectUi, wizard: &NewProject) {
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
    show_colors(ui);
}

/// O quadradinho ao lado de cada código mostra a cor digitada (ou a última que valeu).
fn show_colors(ui: &NewProjectUi) {
    let swatch =
        |text: SharedString| parse_color(&text).map(|c| slint::Color::from_rgb_u8(c.r, c.g, c.b));
    if let Some(c) = swatch(ui.get_background()) {
        ui.set_background_color(c);
    }
    if let Some(c) = swatch(ui.get_border_color()) {
        ui.set_border_swatch(c);
    }
}

/// Clique no quadro de cores: a cor do ponto vira o código do campo (0 = fundo, 1 = borda).
fn color_at(app: &mut App, which: i32, x: f32, y: f32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let hex: SharedString = color_hex(picker_color(f64::from(x), f64::from(y))).into();
    if which == 0 {
        ui.set_background(hex);
    } else {
        ui.set_border_color(hex);
    }
    mount_changed(app);
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
    show_colors(&ui);
    let look = read_look(&ui);
    ui.set_look_error(look.as_ref().err().copied().unwrap_or_default().into());
    if let Ok((look, gap)) = look {
        wizard.look = look;
        wizard.gap = gap;
    }
    crate::new_project::refresh(app);
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

pub fn connect(ui: &NewProjectUi, app: &crate::new_project::Shared) {
    let a = app.clone();
    ui.on_mount_changed(move || mount_changed(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_color_at(move |which, x, y| color_at(&mut a.borrow_mut(), which, x, y));
}
