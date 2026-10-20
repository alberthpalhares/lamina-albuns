//! Fotos nos quadros pela janela (spec FQ): soltar fotos e quadros na lâmina, seleção no
//! navegador, mover lâminas na faixa e cortes na timeline. As decisões são do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::edits::timeline;
use lamina_editor::frame_at;

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, TimelineItem};

type Shared = Rc<RefCell<App>>;

/// O quadro da lâmina atual sob o ponto solto (frações da largura da lâmina).
fn target(app: &App, fx: f32, fy: f32) -> Option<usize> {
    let s = app.session()?;
    frame_at(s.project(), s.current(), f64::from(fx), f64::from(fy))
}

fn after_edit(app: &mut App, changed: bool) {
    if changed {
        app.refresh_all();
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_photo_pressed(move |i, ctrl, shift| {
        let edit = |app: &mut App| {
            let shown = app.shown_photos();
            if let Ok(index) = usize::try_from(i) {
                app.editor.click_photo(&shown, index, ctrl, shift);
            }
            true
        };
        let show = |app: &mut App| {
            let id = app.photo_at(i);
            app.select_photo(id);
            app.refresh_current();
        };
        a.borrow_mut().timed("Clicar numa foto", edit, show);
    });
    let a = app.clone();
    window.on_drop_photo(move |i, fx, fy| {
        let mut app = a.borrow_mut();
        let (Some(photo), Some(frame)) = (app.photo_at(i), target(&app, fx, fy)) else {
            return;
        };
        app.timed(
            "Soltar foto",
            |app| app.editor.drop_photo(frame, photo),
            App::refresh_all,
        );
    });
    let a = app.clone();
    window.on_drop_frame(move |from, fx, fy| {
        let mut app = a.borrow_mut();
        let (Ok(from), Some(to)) = (usize::try_from(from), target(&app, fx, fy)) else {
            return;
        };
        app.timed(
            "Trocar fotos",
            |app| app.editor.drop_frame(from, to),
            App::refresh_all,
        );
    });
    let a = app.clone();
    window.on_frame_selected(move |i| {
        a.borrow_mut().timed(
            "Selecionar quadro",
            |app| {
                app.editor.select_frame(usize::try_from(i).ok());
                true
            },
            App::refresh_current,
        );
    });
    let a = app.clone();
    window.on_move_spread(move |from, to| {
        let mut app = a.borrow_mut();
        let (Ok(from), Ok(to)) = (usize::try_from(from), usize::try_from(to)) else {
            return;
        };
        let moved = app.editor.session_mut().is_some_and(|s| s.go_to(from))
            && (from == to || app.editor.move_current_to(to));
        after_edit(&mut app, moved);
    });
    let a = app.clone();
    window.on_cut_moved(move |k, delta| {
        let mut app = a.borrow_mut();
        let changed = usize::try_from(k).is_ok_and(|k| app.editor.drag_cut(k, delta));
        after_edit(&mut app, changed);
    });
}

/// A timeline: as fotos de cada lâmina e um corte entre uma lâmina e a outra (regra 8).
pub fn refresh_timeline(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let Some(project) = app.session().map(|s| s.project().clone()) else {
        return;
    };
    let thumbs = app.thumbs(&project);
    let spreads = timeline(&project);
    let last = spreads.len().saturating_sub(1);
    let mut items = Vec::new();
    for (k, photos) in spreads.iter().enumerate() {
        for id in photos {
            items.push(TimelineItem {
                img: thumbs.get(id).cloned().unwrap_or_default(),
                cut: false,
                spread: i32::try_from(k).unwrap_or(0),
            });
        }
        if k < last {
            items.push(TimelineItem {
                img: slint::Image::default(),
                cut: true,
                spread: i32::try_from(k).unwrap_or(0),
            });
        }
    }
    w.set_timeline(model(items));
}
