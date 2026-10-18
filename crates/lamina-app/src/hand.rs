//! Quadros à mão pela janela (spec TX): as alças e o arrasto do texto viram o gesto do editor,
//! em mm, e as guias que pegaram no ímã voltam como linhas. As decisões são do editor.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lamina_core::Project;
use lamina_editor::Grip;
use slint::{Model, ModelRc, VecModel};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, BoxData, FrameData};

type Shared = Rc<RefCell<App>>;

/// Distância do ímã na tela, em pixels (regra 7).
const MAGNET_PX: f64 = 8.0;

thread_local! {
    // Onde o gesto começou (pixels da janela) e a largura da lâmina em pixels.
    static START: Cell<Option<(f32, f32, f32)>> = const { Cell::new(None) };
}

/// A alça pelo número da tela: 0 = mover; 1 a 8 a partir do canto de cima à esquerda, no
/// sentido do relógio.
fn grip_of(n: i32) -> Grip {
    let resize = |left, right, top, bottom| Grip::Resize {
        left,
        right,
        top,
        bottom,
    };
    match n {
        1 => resize(true, false, true, false),
        2 => resize(false, false, true, false),
        3 => resize(false, true, true, false),
        4 => resize(false, true, false, false),
        5 => resize(false, true, false, true),
        6 => resize(false, false, false, true),
        7 => resize(true, false, false, true),
        8 => resize(true, false, false, false),
        _ => Grip::Move,
    }
}

/// Milímetros por pixel numa lâmina desenhada com `width_px` pixels.
fn mm_per_px(app: &App, width_px: f32) -> Option<f64> {
    let s = app.editor.session()?;
    let canvas = s.project().spec.canvas_rect();
    (width_px > 0.0).then(|| canvas.width.get() / f64::from(width_px))
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_frame_grab(move |frame, grip, x, y, width| {
        let mut app = a.borrow_mut();
        if usize::try_from(frame).is_ok_and(|f| app.editor.frame_grab(f, grip_of(grip))) {
            START.set(Some((x, y, width)));
            // Só a lâmina: o painel muda no fim do gesto, sem mexer no tamanho da tela agora.
            crate::crop::refresh_spread(&mut app);
        }
    });
    let a = app.clone();
    window.on_frame_drag(move |x, y, alt| {
        let mut app = a.borrow_mut();
        let Some((x0, y0, width)) = START.get() else {
            return;
        };
        let Some(k) = mm_per_px(&app, width) else {
            return;
        };
        let (dx, dy) = (f64::from(x - x0) * k, f64::from(y - y0) * k);
        if app.editor.frame_drag(dx, dy, !alt, MAGNET_PX * k) {
            crate::crop::refresh_spread(&mut app);
        }
    });
    let a = app.clone();
    window.on_frame_release(move || {
        let mut app = a.borrow_mut();
        START.set(None);
        if app.editor.frame_release() {
            app.refresh_all();
        } else {
            app.refresh_current();
        }
    });
}

/// As linhas do ímã em frações da largura da lâmina, para a tela.
pub fn snap_boxes(app: &App, project: &Project) -> ModelRc<BoxData> {
    let canvas = project.spec.canvas_rect();
    let w = canvas.width.get();
    let h = canvas.height.get() / w;
    // Frações de tela cabem com folga em f32.
    #[allow(clippy::cast_possible_truncation)]
    let f = |v: f64| v as f32;
    let lines = app.editor.snap_lines().iter().map(|l| {
        if l.vertical {
            let x = (l.at - canvas.x.get()) / w;
            BoxData {
                x: f(x),
                y: 0.0,
                w: 0.0,
                h: f(h),
            }
        } else {
            let y = (l.at - canvas.y.get()) / w;
            BoxData {
                x: 0.0,
                y: f(y),
                w: 1.0,
                h: 0.0,
            }
        }
    });
    model(lines.collect())
}

thread_local! {
    // Os quadros da lâmina grande: a mesma lista, atualizada linha a linha. Trocar a lista
    // inteira recriaria os quadros da tela e soltaria o mouse no meio do gesto.
    static FRAMES: Rc<VecModel<FrameData>> = Rc::new(VecModel::default());
}

/// Põe `frames` na lista que a lâmina grande usa, mudando só as linhas diferentes.
pub fn stable_frames(frames: &ModelRc<FrameData>) -> ModelRc<FrameData> {
    FRAMES.with(|m| {
        let new: Vec<FrameData> = frames.iter().collect();
        if m.row_count() == new.len() {
            for (i, f) in new.into_iter().enumerate() {
                if m.row_data(i).as_ref() != Some(&f) {
                    m.set_row_data(i, f);
                }
            }
        } else {
            m.set_vec(new);
        }
        ModelRc::from(m.clone())
    })
}
