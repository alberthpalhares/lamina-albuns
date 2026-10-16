//! Recorte pela janela (spec RC): a lâmina atual com o recorte provisório, a prévia, o DPI
//! efetivo do quadro e os gestos. Durante o arrasto só a lâmina é redesenhada, para acompanhar o
//! mouse; a faixa e o painel atualizam quando o gesto é gravado.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_core::{FrameContent, PhotoId, Project};
use lamina_editor::{Area, FrameKind, frame_dpi_text, load_original, photo_extent, spread_view};
use slint::{Timer, TimerMode};

use crate::AppWindow;
use crate::convert::{boxed, spread_data};
use crate::state::{App, Msg};

type Shared = Rc<RefCell<App>>;

/// Giro da roda, em pixels, que vale um passo de zoom (um entalhe da roda do mouse).
const WHEEL_NOTCH: f64 = 120.0;

/// Pausa da roda do mouse que fecha o gesto de zoom (regra 4).
const WHEEL_PAUSE: Duration = Duration::from_millis(500);

/// A foto do quadro `frame` da lâmina `index`.
fn photo_in(project: &Project, index: usize, frame: usize) -> Option<PhotoId> {
    match &project.spreads.get(index)?.frames.get(frame)?.content {
        FrameContent::Photo { photo, .. } => Some(*photo),
        _ => None,
    }
}

/// No modo recorte, pede a foto original (fora da linha da janela); fora dele, solta (regra 9).
fn sync_original(app: &mut App, project: &Project, index: usize) {
    let wanted = app
        .editor
        .crop_mode()
        .and_then(|f| photo_in(project, index, f));
    match wanted {
        None => {
            app.images.clear_original();
            app.loading = None;
        }
        Some(id) if app.images.original_id() != Some(id) && app.loading != Some(id) => {
            app.loading = Some(id);
            let (project, tx) = (project.clone(), app.tx.clone());
            std::thread::spawn(move || {
                if let Some(image) = load_original(&project, id) {
                    // Se a janela já fechou, não há a quem avisar.
                    let _ = tx.send(Msg::Original(id, image));
                }
            });
        }
        Some(_) => {}
    }
}

/// A lâmina atual (com o recorte provisório), a prévia e o DPI do quadro selecionado.
pub fn refresh_spread(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let Some(session) = app.editor.session() else {
        return;
    };
    let index = session.current();
    let (missing, warning) = (
        session.missing().to_vec(),
        !session.warnings_for(index).is_empty(),
    );
    let Some(project) = app.editor.view_project().map(Cow::into_owned) else {
        return;
    };
    sync_original(app, &project, index);
    let images = app.images.spread(&project, index, &app.cache);
    let image = |id: PhotoId| images.get(&id).cloned();
    w.set_current(spread_data(&project, index, &missing, &image, warning));
    w.set_current_index(i32::try_from(index).unwrap_or(0));
    let as_i32 = |f: Option<usize>| f.and_then(|f| i32::try_from(f).ok()).unwrap_or(-1);
    w.set_selected_frame(as_i32(app.editor.selected_frame()));
    w.set_crop_frame(as_i32(app.editor.crop_mode()));

    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    let preview = app.editor.crop_mode().and_then(|f| {
        let view = spread_view(&project, index, area, &missing)?;
        let frame = view.frames.get(f)?;
        let FrameKind::Photo { id, .. } = &frame.kind else {
            return None;
        };
        Some((boxed(photo_extent(frame)?), images.get(id)?.clone()))
    });
    w.set_has_preview(preview.is_some());
    if let Some((rect, img)) = preview {
        w.set_preview(rect);
        w.set_preview_img(img);
    }
    let dpi = app
        .editor
        .selected_frame()
        .and_then(|f| frame_dpi_text(&project, index, f));
    w.set_info_frame(
        dpi.as_ref()
            .map(|(t, _)| t.clone())
            .unwrap_or_default()
            .into(),
    );
    w.set_info_frame_warn(dpi.is_some_and(|(_, warn)| warn));
    let hint = if app.editor.crop_mode().is_some() {
        "Roda do mouse ou + e − para o zoom · arraste para mover · 0 encaixa · Esc sai"
    } else if app.editor.selected_frame().is_some() {
        "Delete tira a foto · C ou duplo clique recorta · arraste para outro quadro para trocar"
    } else {
        ""
    };
    w.set_hint(hint.into());
}

/// A foto original chegou: entra no quadro se ele ainda está em modo recorte.
pub fn original_ready(
    app: &mut App,
    id: PhotoId,
    image: std::sync::Arc<lamina_editor::screens::Rgba>,
) {
    if app.loading == Some(id) {
        app.images.set_original(id, &image);
        app.loading = None;
        refresh_spread(app);
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_frame_double(move |i| {
        let mut app = a.borrow_mut();
        if let Ok(frame) = usize::try_from(i)
            && app.editor.enter_crop(frame)
        {
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.on_crop_drag(move |dx, dy| {
        let mut app = a.borrow_mut();
        if app.editor.crop_drag(f64::from(dx), f64::from(dy)) {
            refresh_spread(&mut app);
        }
    });
    let a = app.clone();
    window.on_crop_end(move || {
        let mut app = a.borrow_mut();
        if app.editor.commit_crop() {
            app.refresh_all();
        }
    });
    let a = app.clone();
    window.on_crop_wheel(move |delta| {
        let mut app = a.borrow_mut();
        let steps = f64::from(delta) / WHEEL_NOTCH;
        let factor = lamina_editor::editor::ZOOM_STEP.powf(steps);
        if !app.editor.crop_zoom(factor) {
            return;
        }
        refresh_spread(&mut app);
        let b = a.clone();
        app.crop_timer
            .start(TimerMode::SingleShot, WHEEL_PAUSE, move || {
                let mut app = b.borrow_mut();
                if app.editor.commit_crop() {
                    app.refresh_all();
                }
            });
    });
}

/// Um temporizador parado, para o `App` guardar o da roda do mouse.
pub fn timer() -> Timer {
    Timer::default()
}
