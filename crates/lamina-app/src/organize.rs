//! O modo organizar pela janela (spec EG): os cliques com Ctrl e Shift, o laço, o "Concluir" e o
//! `OrganizeUi` com o título, a seleção e a caixa do grupo. As decisões são do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::{Area, Command, PxRect, spread_view};
use slint::ComponentHandle;

use crate::convert::{boxed, model};
use crate::state::App;
use crate::{AppWindow, OrganizeUi};

type Shared = Rc<RefCell<App>>;

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_frame_clicked(move |i, ctrl, shift| {
        let Ok(frame) = usize::try_from(i) else {
            return;
        };
        a.borrow_mut().timed(
            "Selecionar quadro",
            |app| {
                app.editor.organize_click(frame, ctrl, shift);
                true
            },
            App::refresh_current,
        );
    });
    let a = app.clone();
    window.on_lasso_done(move |x0, y0, x1, y1, shift| {
        let mut app = a.borrow_mut();
        let p = |x: f32, y: f32| (f64::from(x), f64::from(y));
        app.editor.lasso(p(x0, y0), p(x1, y1), shift);
        app.refresh_current();
    });
    let a = app.clone();
    window
        .global::<OrganizeUi>()
        .on_done(move || crate::input::run_command(&a, Command::Organize));
}

/// O título, a seleção e a caixa do grupo do modo organizar.
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<OrganizeUi>();
    ui.set_on(app.editor.organizing());
    ui.set_title(app.editor.organize_title().into());
    ui.set_count(app.editor.selection_text().into());
    let rects = selected_rects(app);
    let group = rects.iter().copied().reduce(union);
    ui.set_has_group(rects.len() > 1);
    ui.set_group(group.map(boxed).unwrap_or_default());
    ui.set_boxes(model(rects.into_iter().map(boxed).collect()));
}

/// Os retângulos dos quadros selecionados, em frações da largura da lâmina.
fn selected_rects(app: &App) -> Vec<PxRect> {
    let (Some(s), Some(project)) = (app.session(), app.editor.view_project()) else {
        return Vec::new();
    };
    if !app.editor.organizing() {
        return Vec::new();
    }
    // Com o gesto do grupo aplicado: as marcas acompanham o mouse.
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    let Some(view) = spread_view(&project, s.current(), area, &[]) else {
        return Vec::new();
    };
    app.editor
        .selected_frames()
        .into_iter()
        .filter_map(|f| view.frames.get(f).map(|v| v.rect))
        .collect()
}

/// A caixa que envolve os retângulos (vazia sem nenhum).
pub fn union_all(rects: &[PxRect]) -> PxRect {
    rects.iter().copied().reduce(union).unwrap_or_default()
}

fn union(a: PxRect, b: PxRect) -> PxRect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    PxRect {
        x,
        y,
        width: (a.x + a.width).max(b.x + b.width) - x,
        height: (a.y + a.height).max(b.y + b.height) - y,
    }
}
