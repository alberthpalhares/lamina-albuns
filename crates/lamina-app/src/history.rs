//! O painel de histórico e os rótulos do menu Editar (spec HI).

use std::cell::RefCell;
use std::rc::Rc;

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, HistoryItem};

pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let Some(session) = app.editor.session() else {
        w.set_history(model(Vec::new()));
        w.set_undo_text("".into());
        w.set_redo_text("".into());
        return;
    };
    let list = session.history_list();
    let start = HistoryItem {
        label: list.start.clone().into(),
        undone: false,
        saved: list.start_saved,
    };
    let steps = list.steps.iter().map(|s| HistoryItem {
        label: s.label.clone().into(),
        undone: s.undone,
        saved: s.saved,
    });
    w.set_history(model(std::iter::once(start).chain(steps).collect()));
    w.set_history_pos(i32::try_from(list.position).unwrap_or(0));
    w.set_undo_text(session.undo_label().unwrap_or_default().into());
    w.set_redo_text(session.redo_label().unwrap_or_default().into());
}

pub fn connect(window: &AppWindow, app: &Rc<RefCell<App>>) {
    let a = app.clone();
    window.on_history_go(move |i| {
        let mut app = a.borrow_mut();
        if usize::try_from(i).is_ok_and(|step| app.editor.go_to_step(step)) {
            app.refresh_all();
        }
    });
}
