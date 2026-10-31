//! O movimento do arrasto (spec MV, regra 12): o fantasma (a imagem vem de `ghost.rs`), o quadro
//! de destino e as lâminas abrindo espaço na faixa. Onde a soltura cai e o que ela faz ficam em
//! `drops.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::motion::{edge_speed, strip_gap, strip_landing, strip_shift};
use slint::{ComponentHandle, Model};

use crate::AppWindow;
use crate::convert::to_f32;
use crate::state::App;

type Shared = Rc<RefCell<App>>;

pub fn connect_motion(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_drag_started(move |kind, from| {
        let mut app = a.borrow_mut();
        if let Some(ghost) = crate::ghost::make(&mut app, kind, from)
            && let Some(w) = app.window.upgrade()
        {
            w.set_ghost(ghost);
        }
    });
    let a = app.clone();
    window.on_drop_at(move |fx, fy| {
        let app = a.borrow();
        // O mesmo alvo da soltura (spec ML, regra 7): só quadros de foto acendem.
        let target = app.session().and_then(|s| {
            lamina_editor::motion::cross_target(
                s.project(),
                s.current(),
                f64::from(fx),
                f64::from(fy),
            )
        });
        match target {
            Some(lamina_editor::motion::CrossHit::Frame { rect, .. }) => {
                crate::convert::boxed(rect)
            }
            // Numa vizinha do rolo, o quadro dela, em frações da lâmina grande (spec ML).
            _ => crate::roll_connect::neighbor_target(&app, f64::from(fx), f64::from(fy))
                .and_then(|(_, rect)| rect)
                .map(crate::convert::boxed)
                .unwrap_or_default(),
        }
    });
    connect_strip(window);
}

/// As larguras das miniaturas da faixa, em px lógicos, como a faixa as desenha (84 px de altura
/// pela proporção, mais a borda).
fn strip_widths(w: &AppWindow) -> Vec<f64> {
    // A faixa pode estar compacta (spec DJ, regra 12): 44 px são a borda, o número e os vãos.
    let thumb = f64::from(w.get_strip_height()) - 44.0;
    w.get_spreads()
        .iter()
        .map(|s| thumb / f64::from(s.aspect.max(0.01)) + 6.0)
        .collect()
}

fn index(v: i32) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// As contas do arrasto na faixa de lâminas (spec FX, regras 1 a 4), pelas larguras reais.
fn connect_strip(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_strip_gap(move |from, center| {
        let Some(w) = weak.upgrade() else {
            return from;
        };
        let gap = strip_gap(&strip_widths(&w), index(from), f64::from(center));
        i32::try_from(gap).unwrap_or(from)
    });
    let weak = window.as_weak();
    window.on_strip_shift(move |from, gap, i| {
        let Some(w) = weak.upgrade() else {
            return 0.0;
        };
        let widths = strip_widths(&w);
        to_f32(strip_shift(&widths, index(from), index(gap), index(i)))
    });
    let weak = window.as_weak();
    window.on_strip_landing(move |from, gap| {
        let Some(w) = weak.upgrade() else {
            return 0.0;
        };
        to_f32(strip_landing(&strip_widths(&w), index(from), index(gap)))
    });
    window.on_strip_edge(|x, width| {
        // A velocidade em lâminas por segundo do rolo vira px por segundo pela largura de uma
        // miniatura da faixa (~ 160 px).
        to_f32(edge_speed(f64::from(x), f64::from(width)) * 160.0)
    });
}
