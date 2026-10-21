//! O movimento do arrasto (spec MV, regra 12): o fantasma (a imagem vem de `ghost.rs`), o quadro
//! de destino e as lâminas abrindo espaço na faixa. Onde a soltura cai e o que ela faz ficam em
//! `drops.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::motion::reorder;

use crate::AppWindow;
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
    let a = app.clone();
    window.on_spread_shift(move |from, gap, i| {
        let app = a.borrow();
        let count = app.session().map_or(0, |s| s.spread_count());
        let (Ok(from), Ok(gap), Ok(i)) = (
            usize::try_from(from),
            usize::try_from(gap),
            usize::try_from(i),
        ) else {
            return 0;
        };
        reorder(from, gap, count).shift.get(i).copied().unwrap_or(0)
    });
}
