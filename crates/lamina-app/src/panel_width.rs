//! A borda do painel da direita (spec FX, regras 13 a 15): o arrasto mostra a largura presa à
//! faixa da janela sem animar a lâmina; o soltar e o clique duplo guardam no `janela.json`.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::panel_width::panel_width;
use slint::ComponentHandle;

use crate::AppWindow;
use crate::state::App;

type Shared = Rc<RefCell<App>>;

/// A largura da janela em px lógicos.
fn window_px(w: &AppWindow) -> u32 {
    let window = w.window();
    let logical = f64::from(window.size().width) / f64::from(window.scale_factor().max(0.1));
    // Larguras de janela cabem com folga.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = logical.max(0.0) as u32;
    px
}

fn px(v: f32) -> u32 {
    // Larguras de painel cabem com folga.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = v.max(0.0).round() as u32;
    px
}

/// A largura guardada, presa à janela de agora.
pub fn refresh(app: &App) {
    if let Some(w) = app.window.upgrade() {
        let shown = app.editor.panel_width(window_px(&w));
        #[allow(clippy::cast_precision_loss)]
        w.set_panel_width(shown as f32);
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_panel_width_drag(move |width| {
        let mut app = a.borrow_mut();
        crate::waits::resized(&mut app);
        if let Some(w) = app.window.upgrade() {
            let shown = panel_width(Some(px(width)), window_px(&w));
            #[allow(clippy::cast_precision_loss)]
            w.set_panel_width(shown as f32);
        }
    });
    let a = app.clone();
    window.on_panel_width_set(move |width| set(&a, Some(px(width))));
    let a = app.clone();
    window.on_panel_width_reset(move || set(&a, None));
}

fn set(app: &Shared, width: Option<u32>) {
    let mut app = app.borrow_mut();
    let Some(w) = app.window.upgrade() else {
        return;
    };
    crate::waits::resized(&mut app);
    let shown = app.editor.set_panel_width(width, window_px(&w));
    #[allow(clippy::cast_precision_loss)]
    w.set_panel_width(shown as f32);
}
