//! Interface gráfica do Lâmina (spec `janela-principal`; ADR 0002).
//!
//! Este binário só traduz (regra 1): as telas estão em `ui/*.slint` e toda decisão vem do
//! `lamina-editor`. O trabalho longo roda em outras linhas e volta por um canal, lido a cada
//! 50 ms pela linha da janela.

#![windows_subsystem = "windows"]

mod convert;
mod crop;
mod drag;
mod export;
mod files;
mod history;
mod images;
mod look;
mod new_album;
mod panel;
mod relink;
mod state;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::Duration;

use lamina_editor::{
    Cache, Command, Editor, Filter, Key, PREFS_FILE, Part, PrefsStore, STRIPS_FILE, Shortcut,
    StripsStore, config_dir,
};
use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};

use crate::state::{App, Pending};

/// As telas compiladas pelo Slint. É código gerado: só aqui o `unsafe` é liberado.
#[allow(
    unsafe_code,
    missing_debug_implementations,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
mod ui {
    slint::include_modules!();
}

pub use ui::*;

type Shared = Rc<RefCell<App>>;

/// Roda um comando do registro (menu, botões e teclado passam por aqui).
fn run_command(app: &Shared, command: Command) {
    let mut a = app.borrow_mut();
    // O menu também obedece: no meio de um trabalho em segundo plano, nada além de "Sobre".
    if !a.editor.available(command) {
        return;
    }
    match command {
        Command::NewAlbum => files::request(&mut a, Pending::NewAlbum),
        Command::Open => files::request(&mut a, Pending::Open),
        Command::Save => {
            files::save(&mut a, false);
        }
        Command::SaveAs => {
            files::save(&mut a, true);
        }
        Command::Export => export::start(&mut a),
        Command::FindMissing => relink::find(&mut a),
        Command::About => {
            if let Some(w) = a.window.upgrade() {
                w.set_show_about(true);
            }
        }
        _ => {
            // Navegar e as guias só mudam a lâmina mostrada; o resto edita o álbum e muda também
            // a faixa, a timeline e os usos no navegador.
            let only_view = matches!(
                command,
                Command::NextSpread
                    | Command::PreviousSpread
                    | Command::FirstSpread
                    | Command::LastSpread
                    | Command::ToggleGuides
                    | Command::ToggleSpreads
                    | Command::ToggleTimeline
                    | Command::TogglePhotos
            );
            if a.editor.run(command) {
                if only_view {
                    a.refresh_current();
                } else {
                    a.refresh_all();
                }
            }
        }
    }
}

fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_command(move |name| {
        if let Some(command) = Command::from_name(&name) {
            run_command(&a, command);
        }
    });
    let a = app.clone();
    window.on_key(move |name, ctrl, shift, alt| {
        let Some(key) = Key::from_name(&name) else {
            return false;
        };
        let shortcut = Shortcut {
            ctrl,
            shift,
            alt,
            key,
        };
        let Some(command) = Command::from_shortcut(shortcut) else {
            return false;
        };
        if a.borrow().editor.available(command) {
            run_command(&a, command);
        }
        true
    });
    let a = app.clone();
    window.on_go_to(move |index| {
        let mut app = a.borrow_mut();
        let moved = usize::try_from(index).is_ok_and(|i| app.editor.go_to_spread(i));
        if moved {
            app.refresh_current();
        }
    });
    connect_photos(window, app);
    connect_dialogs(window, app);
    connect_layouts(window, app);
    drag::connect(window, app);
    crop::connect(window, app);
    look::connect(window, app);
    history::connect(window, app);
}

/// O painel de layouts: clique aplica; a estrela e o "×" marcam a opção (spec TL, seção 3).
fn connect_layouts(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_layout_apply(move |i| {
        let mut app = a.borrow_mut();
        if usize::try_from(i).is_ok_and(|i| app.editor.apply_option(Part::Spread, i)) {
            app.refresh_strip();
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.on_layout_favorite(move |i| mark(&a, i, true));
    let a = app.clone();
    window.on_layout_reject(move |i| mark(&a, i, false));
    let a = app.clone();
    window.on_layout_list_changed(move |list| {
        let mut app = a.borrow_mut();
        if (list == 1) != app.editor.reordered() && app.editor.run(Command::ToggleReordered) {
            app.refresh_current();
        }
    });
}

fn mark(app: &Shared, i: i32, favorite: bool) {
    let mut app = app.borrow_mut();
    if usize::try_from(i).is_ok_and(|i| app.editor.mark_option(i, favorite)) {
        app.refresh_strip();
        app.refresh_current();
    }
}

fn connect_photos(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_photo_clicked(move |index| {
        let mut app = a.borrow_mut();
        let id = app.photo_at(index);
        app.select_photo(id);
    });
    let a = app.clone();
    window.on_photo_double_clicked(move |index| {
        let mut app = a.borrow_mut();
        if let Some(id) = app.photo_at(index)
            && app.editor.go_to_photo(id)
        {
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.on_filter_changed(move |filter| {
        let filter = match filter {
            1 => Filter::Used,
            2 => Filter::Unused,
            _ => Filter::All,
        };
        a.borrow_mut().set_filter(filter);
    });
}

fn connect_dialogs(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_na_changed(move || new_album::changed(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_na_create(move || new_album::create(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_confirm(move |choice| files::answer(&mut a.borrow_mut(), choice));
    connect_export(window, app);
    let a = app.clone();
    window.window().on_close_requested(move || {
        let mut app = a.borrow_mut();
        if app.editor.needs_confirmation() {
            files::request(&mut app, Pending::Close);
            CloseRequestResponse::KeepWindowShown
        } else {
            CloseRequestResponse::HideWindow
        }
    });
}

/// A janela Exportar e o resumo com "Abrir a pasta" (spec EI).
fn connect_export(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_ex_pick_folder(move || export::pick_folder(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_changed(move || export::changed(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_export(move || export::export(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_go_to(move |i| export::go_to(&mut a.borrow_mut(), i));
    let a = app.clone();
    window.on_ex_ignore(move |i| export::ignore(&mut a.borrow_mut(), i));
    let a = app.clone();
    window.on_ex_proceed(move || export::proceed(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_replace(move || export::run(&mut a.borrow_mut(), true));
    let a = app.clone();
    window.on_ex_back(move || export::back(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_open_folder(move || export::open_folder(&a.borrow()));
}

fn main() -> anyhow::Result<()> {
    let window = AppWindow::new()?;
    window.set_version(env!("CARGO_PKG_VERSION").into());
    let (tx, rx) = channel();
    let prefs = PrefsStore::load(&config_dir().join(PREFS_FILE));
    let mut editor = Editor::with_prefs(prefs);
    editor.set_strips(StripsStore::load(&config_dir().join(STRIPS_FILE)));
    let app: Shared = Rc::new(RefCell::new(App::new(&window, Cache::system(), editor, tx)));
    connect(&window, &app);
    app.borrow_mut().refresh_all();
    // `lamina-app projeto.lamina` abre o projeto direto (e, no futuro, o duplo clique no arquivo).
    if let Some(file) = std::env::args_os().nth(1) {
        files::open_file(&mut app.borrow_mut(), std::path::Path::new(&file));
    }

    let a = app.clone();
    let poll = Timer::default();
    poll.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        while let Ok(msg) = rx.try_recv() {
            new_album::message(&mut a.borrow_mut(), msg);
        }
    });
    window.invoke_focus_main();
    window.run()?;
    Ok(())
}
