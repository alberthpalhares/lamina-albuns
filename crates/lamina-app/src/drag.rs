//! Fotos nos quadros pela janela (spec FQ): soltar fotos e quadros na lâmina, seleção no
//! navegador, mover lâminas na faixa e cortes na timeline. As decisões são do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::motion::reorder;
use lamina_editor::motion::{TimelineKey, timeline_rows};
use lamina_editor::{DropPlace, frame_at};

use crate::state::App;
use crate::{AppWindow, TimelineItem};

type Shared = Rc<RefCell<App>>;

/// O quadro da lâmina atual sob o ponto solto (frações da largura da lâmina).
fn target(app: &App, fx: f32, fy: f32) -> Option<usize> {
    let s = app.session()?;
    frame_at(s.project(), s.current(), f64::from(fx), f64::from(fy))
}

/// O ponto solto cai dentro da lâmina grande (frações da largura; a altura é a proporção)?
fn inside(app: &App, fx: f32, fy: f32) -> bool {
    let Some(s) = app.session() else {
        return false;
    };
    if s.spread_count() == 0 {
        return false;
    }
    let aspect = crate::convert::aspect(s.project());
    (0.0..=1.0).contains(&fx) && (0.0..=aspect).contains(&fy)
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
        let Some(photo) = app.photo_at(i) else {
            return;
        };
        // Sobre um quadro, como sempre (FQ); na lâmina, fora dos quadros, as fotos entram nela; no
        // álbum ainda sem lâminas, viram a primeira (spec IM, regras 7 e 8).
        let place = match (target(&app, fx, fy), inside(&app, fx, fy)) {
            (Some(frame), _) => DropPlace::Frame(frame),
            (None, true) => DropPlace::Spread,
            (None, false) if app.session().is_some_and(|s| s.spread_count() == 0) => {
                DropPlace::NewSpread(0)
            }
            (None, false) => return,
        };
        let photos = match place {
            DropPlace::Frame(_) => vec![photo],
            _ => app.editor.drag_photos(photo),
        };
        app.timed(
            "Soltar fotos",
            |app| app.editor.drop_photos(place, &photos),
            App::refresh_all,
        );
    });
    let a = app.clone();
    window.on_drop_photos_gap(move |i, gap| {
        let mut app = a.borrow_mut();
        let (Some(photo), Ok(gap)) = (app.photo_at(i), usize::try_from(gap)) else {
            return;
        };
        let photos = app.editor.drag_photos(photo);
        app.timed(
            "Lâmina nova com fotos",
            |app| app.editor.drop_photos(DropPlace::NewSpread(gap), &photos),
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
                // Fora dos quadros desmarca (no modo organizar, todos).
                match usize::try_from(i) {
                    Ok(frame) => app.editor.select_frame(Some(frame)),
                    Err(_) => app.editor.click_nothing(),
                }
                true
            },
            App::refresh_current,
        );
    });
    let a = app.clone();
    window.on_move_spread(move |from, gap| {
        let mut app = a.borrow_mut();
        let (Ok(from), Ok(gap)) = (usize::try_from(from), usize::try_from(gap)) else {
            return;
        };
        // O vão vira a posição final pela regra do editor (spec MV, MV-B23).
        let count = app.session().map_or(0, |s| s.spread_count());
        let to = reorder(from, gap, count).to;
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

/// A timeline: as fotos de cada lâmina e um corte entre uma lâmina e a outra (regra 8), só as
/// posições que mudaram (spec MV, regra 3).
pub fn refresh_timeline(app: &mut App) {
    let Some(project) = app.session().map(|s| s.project().clone()) else {
        return;
    };
    let thumbs = app.thumbs(&project);
    let rows = timeline_rows(&project);
    let items: Vec<TimelineItem> = rows
        .iter()
        .map(|(key, spread)| {
            let spread = i32::try_from(*spread).unwrap_or(0);
            match key {
                TimelineKey::Photo(id, _) => TimelineItem {
                    img: thumbs.get(id).cloned().unwrap_or_default(),
                    cut: false,
                    spread,
                },
                TimelineKey::Cut(_) => TimelineItem {
                    img: slint::Image::default(),
                    cut: true,
                    spread,
                },
            }
        })
        .collect();
    app.lists.line.sync(rows, |i| items[i].clone());
}

/// O arrasto (spec MV, regra 12): o fantasma, o quadro de destino e as lâminas abrindo espaço.
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
        let target = app.session().and_then(|s| {
            lamina_editor::motion::drop_target(
                s.project(),
                s.current(),
                f64::from(fx),
                f64::from(fy),
            )
        });
        target
            .map(|(_, r)| crate::convert::boxed(r))
            .unwrap_or_default()
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
