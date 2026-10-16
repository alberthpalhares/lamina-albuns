//! O painel de aparência (spec AP): copia a aparência da lâmina para a tela e liga os botões às
//! edições do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_core::{Mm, PageSide};
use lamina_editor::{BAD_COLOR, PALETTE, color_hex, parse_color};
use slint::Color;

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, Swatch};

type Shared = Rc<RefCell<App>>;

/// Passos dos botões − e +, em mm, e os limites (spec AP, seção 3).
const GAP_STEP: f64 = 0.5;
const GAP_MAX: f64 = 20.0;
const BORDER_STEP: f64 = 0.5;
const BORDER_MAX: f64 = 5.0;
const RADIUS_STEP: f64 = 1.0;
const RADIUS_MAX: f64 = 10.0;
/// Borda que uma cor escolhida sem espessura ganha.
const BORDER_FIRST: f64 = 1.0;

fn mm(v: f64) -> String {
    format!("{} mm", (v * 10.0).round() / 10.0).replace('.', ",")
}

fn color(c: lamina_core::Rgb) -> Color {
    Color::from_rgb_u8(c.r, c.g, c.b)
}

fn stepped(value: f64, delta: i32, step: f64, max: f64) -> Mm {
    Mm((value + f64::from(delta) * step).clamp(0.0, max))
}

pub fn refresh(app: &App) {
    let (Some(w), Some(look)) = (app.window.upgrade(), app.editor.look()) else {
        return;
    };
    let palette: Vec<Swatch> = PALETTE
        .iter()
        .map(|(name, c)| Swatch {
            c: color(*c),
            name: (*name).into(),
        })
        .collect();
    w.set_palette(model(palette));
    w.set_look_gap(mm(look.gap.get()).into());
    w.set_look_border(mm(look.border.width.get()).into());
    w.set_look_radius(mm(look.radius.get()).into());
    w.set_look_split(look.split);
    w.set_look_left(color(look.left));
    w.set_look_right(color(look.right));
    w.set_look_border_color(color(look.border.color));
    w.set_look_only_frame(look.only_frame);
    if w.get_look_hex_error().is_empty() {
        w.set_look_hex(color_hex(look.left).into());
    }
}

/// Depois de uma edição: tudo na tela, e o aviso do editor (espaço reduzido), se houver.
fn after(app: &mut App, notice: Option<String>) {
    if let Some(n) = notice {
        app.notice = n;
    }
    app.refresh_all();
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_look_gap_step(move |d| {
        let mut app = a.borrow_mut();
        let Some(look) = app.editor.look() else {
            return;
        };
        let notice = app
            .editor
            .set_gap(stepped(look.gap.get(), d, GAP_STEP, GAP_MAX));
        after(&mut app, notice);
    });
    let a = app.clone();
    window.on_look_border_step(move |d| {
        let mut app = a.borrow_mut();
        let Some(look) = app.editor.look() else {
            return;
        };
        let width = stepped(look.border.width.get(), d, BORDER_STEP, BORDER_MAX);
        app.editor.set_border(width, look.border.color);
        after(&mut app, None);
    });
    let a = app.clone();
    window.on_look_radius_step(move |d| {
        let mut app = a.borrow_mut();
        let Some(look) = app.editor.look() else {
            return;
        };
        app.editor
            .set_radius(stepped(look.radius.get(), d, RADIUS_STEP, RADIUS_MAX));
        after(&mut app, None);
    });
    let a = app.clone();
    window.on_look_background(move |i, side| {
        let mut app = a.borrow_mut();
        let Some((_, c)) = usize::try_from(i).ok().and_then(|i| PALETTE.get(i)) else {
            return;
        };
        let side = match side {
            0 => Some(PageSide::Left),
            1 => Some(PageSide::Right),
            _ => None,
        };
        app.editor.set_background(*c, side);
        after(&mut app, None);
    });
    let a = app.clone();
    window.on_look_background_hex(move |text| {
        let mut app = a.borrow_mut();
        let Some(w) = app.window.upgrade() else {
            return;
        };
        match parse_color(&text) {
            Some(c) => {
                w.set_look_hex_error("".into());
                app.editor.set_background(c, None);
                after(&mut app, None);
            }
            None => w.set_look_hex_error(BAD_COLOR.into()),
        }
    });
    let a = app.clone();
    window.on_look_split_changed(move |on| {
        let mut app = a.borrow_mut();
        let Some(look) = app.editor.look() else {
            return;
        };
        let side = on.then_some(PageSide::Right);
        app.editor.set_background(look.left, side);
        after(&mut app, None);
    });
    let a = app.clone();
    window.on_look_border_pick(move |i| {
        let mut app = a.borrow_mut();
        let (Some(look), Some((_, c))) = (
            app.editor.look(),
            usize::try_from(i).ok().and_then(|i| PALETTE.get(i)),
        ) else {
            return;
        };
        let width = if look.border.width.get() > 0.0 {
            look.border.width
        } else {
            Mm(BORDER_FIRST)
        };
        app.editor.set_border(width, *c);
        after(&mut app, None);
    });
    let a = app.clone();
    window.on_look_apply_all(move || {
        let mut app = a.borrow_mut();
        app.editor.style_all();
        after(&mut app, None);
    });
}
