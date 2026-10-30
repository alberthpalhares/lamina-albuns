//! A lista de fontes da aba Quadro pela janela (spec FS): copia as linhas do editor para a tela,
//! faz as prévias das linhas visíveis fora da linha da janela e solta tudo ao fechar.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_editor::TextChange;
use lamina_editor::font_picker::{
    FontPreviews, FontRow, LOADING, Preview, font_rows, names_ready, preview,
};
use slint::{ComponentHandle, Image, Model, Rgba8Pixel, SharedPixelBuffer, Timer, VecModel};

use crate::state::App;
use crate::{AppWindow, FontRowUi, FontUi};

type Shared = Rc<RefCell<App>>;

/// A lista aberta: a fonte do texto, a busca, as linhas e as prévias já feitas.
#[derive(Default)]
struct List {
    current: String,
    query: String,
    rows: Vec<FontRow>,
    previews: FontPreviews,
}

thread_local! {
    static LIST: RefCell<List> = RefCell::default();
    static ROWS: Rc<VecModel<FontRowUi>> = Rc::new(VecModel::default());
    // Enquanto os nomes do computador não chegam, confere de tempos em tempos (regra 1).
    static WAIT: Timer = Timer::default();
}

fn image(img: &Preview) -> Image {
    let buffer =
        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&img.rgba, img.width, img.height);
    Image::from_rgba8_premultiplied(buffer)
}

fn row_ui(row: &FontRow, previews: &FontPreviews) -> FontRowUi {
    let label = row.label().into();
    match row {
        FontRow::Header(_) => FontRowUi {
            kind: 0,
            label,
            ..FontRowUi::default()
        },
        FontRow::Font {
            name,
            current,
            missing,
        } => {
            let preview = previews.get(name);
            FontRowUi {
                kind: 1,
                label,
                name: name.into(),
                current: *current,
                missing: *missing,
                has_preview: preview.is_some(),
                preview: preview.map(image).unwrap_or_default(),
            }
        }
        FontRow::Note(_) => FontRowUi {
            kind: 2,
            label,
            ..FontRowUi::default()
        },
    }
}

/// Refaz as linhas para a busca de agora e marca a fonte do texto (ou a primeira).
fn rebuild(w: &AppWindow) {
    let picked = LIST.with(|l| {
        let mut l = l.borrow_mut();
        l.rows = font_rows(&l.current, &l.query);
        let items: Vec<FontRowUi> = l.rows.iter().map(|r| row_ui(r, &l.previews)).collect();
        ROWS.with(|m| m.set_vec(items));
        let font = |r: &FontRow| matches!(r, FontRow::Font { .. });
        l.rows
            .iter()
            .position(|r| matches!(r, FontRow::Font { current: true, .. }) && l.query.is_empty())
            .or_else(|| l.rows.iter().position(font))
    });
    w.global::<FontUi>()
        .set_picked(picked.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1));
    wait_for_names(w);
}

/// Com "Procurando…" na lista, refaz quando os nomes chegam, sem precisar reabrir (FS-B11).
fn wait_for_names(w: &AppWindow) {
    let loading = LIST.with(|l| {
        l.borrow()
            .rows
            .iter()
            .any(|r| matches!(r, FontRow::Note(n) if n == LOADING))
    });
    if !loading {
        WAIT.with(Timer::stop);
        return;
    }
    let weak = w.as_weak();
    WAIT.with(|t| {
        t.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(100),
            move || {
                if names_ready()
                    && let Some(w) = weak.upgrade()
                {
                    WAIT.with(Timer::stop);
                    rebuild(&w);
                }
            },
        );
    });
}

/// Faz em segundo plano as prévias das linhas `first..first + count` que ainda não têm (regra 11).
fn visible(w: &AppWindow, first: i32, count: i32) {
    let first = usize::try_from(first).unwrap_or(0);
    let count = usize::try_from(count).unwrap_or(0);
    let wanted = LIST.with(|l| {
        let mut l = l.borrow_mut();
        let names: Vec<String> = l
            .rows
            .iter()
            .skip(first)
            .take(count)
            .filter_map(|r| match r {
                FontRow::Font {
                    name,
                    missing: false,
                    ..
                } => Some(name.clone()),
                _ => None,
            })
            .collect();
        l.previews.wanted(&names)
    });
    if wanted.is_empty() {
        return;
    }
    let weak = w.as_weak();
    let _ = std::thread::Builder::new()
        .name("prévias das fontes".into())
        .spawn(move || {
            for name in wanted {
                let made = preview(&name);
                let _ = weak.upgrade_in_event_loop(move |_| deliver(name, made));
            }
        });
}

/// Uma prévia pronta: guarda e troca só a linha dela.
fn deliver(name: String, made: Option<Preview>) {
    let shown = made.as_ref().map(image);
    LIST.with(|l| l.borrow_mut().previews.put(name.clone(), made));
    let Some(shown) = shown else {
        return;
    };
    ROWS.with(|m| {
        if let Some(i) = m
            .iter()
            .position(|r| r.kind == 1 && r.name == name.as_str())
        {
            let mut row = m.row_data(i).unwrap_or_default();
            row.preview = shown;
            row.has_preview = true;
            m.set_row_data(i, row);
        }
    });
}

/// A próxima linha de fonte a partir de `from`, na direção `delta`; fica onde está no fim.
fn step(from: i32, delta: i32) -> i32 {
    LIST.with(|l| {
        let l = l.borrow();
        let mut i = from;
        loop {
            i += delta;
            let Some(row) = usize::try_from(i).ok().and_then(|i| l.rows.get(i)) else {
                return from;
            };
            if matches!(row, FontRow::Font { .. }) {
                return i;
            }
        }
    })
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<FontUi>();
    ROWS.with(|m| ui.set_rows(m.clone().into()));
    let a = app.clone();
    let w = window.as_weak();
    ui.on_opened(move || {
        let current = a
            .borrow()
            .editor
            .selected_text()
            .map(|t| t.font.clone())
            .unwrap_or_default();
        LIST.with(|l| {
            let mut l = l.borrow_mut();
            l.current = current;
            l.query.clear();
            l.previews.clear();
        });
        if let Some(w) = w.upgrade() {
            rebuild(&w);
        }
    });
    let w = window.as_weak();
    ui.on_search(move |q| {
        LIST.with(|l| l.borrow_mut().query = q.into());
        if let Some(w) = w.upgrade() {
            rebuild(&w);
        }
    });
    ui.on_closed(|| {
        WAIT.with(Timer::stop);
        LIST.with(|l| {
            let mut l = l.borrow_mut();
            l.previews.clear();
            l.rows.clear();
        });
        ROWS.with(|m| m.set_vec(Vec::new()));
    });
    let w = window.as_weak();
    ui.on_visible(move |first, count| {
        if let Some(w) = w.upgrade() {
            visible(&w, first, count);
        }
    });
    let a = app.clone();
    ui.on_chosen(move |name| {
        crate::frame_panel::change(&a, TextChange::Font(name.into()));
    });
    ui.on_step(step);
}
