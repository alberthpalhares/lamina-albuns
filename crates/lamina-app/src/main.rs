//! Interface gráfica do Lamina (spec `janela-principal`; ADR 0002).
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
mod frame_panel;
mod hand;
mod help;
mod history;
mod images;
mod input;
mod install;
mod lists;
mod loader;
mod look;
mod mailbox;
mod models;
mod new_album;
mod panel;
mod presets;
mod recents;
mod refresh;
mod relink;
mod spec_dialog;
mod state;
mod texts;
mod timings;
mod updates;
mod verify;
mod welcome;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_editor::{
    Cache, Command, Editor, Filter, MODELS_FILE, ModelsStore, PREFS_FILE, PRESETS_FILE, Part,
    PrefsStore, PresetsStore, RECENTS_FILE, RecentsStore, STRIPS_FILE, StripsStore, config_dir,
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

fn connect(window: &AppWindow, app: &Shared) {
    input::connect(window, app);
    let a = app.clone();
    window.on_go_to(move |index| {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        a.borrow_mut().timed(
            "Ir para a lâmina",
            |app| app.editor.go_to_spread(index),
            App::refresh_current,
        );
    });
    connect_photos(window, app);
    connect_dialogs(window, app);
    connect_layouts(window, app);
    drag::connect(window, app);
    crop::connect(window, app);
    look::connect(window, app);
    hand::connect(window, app);
    frame_panel::connect(window, app);
    models::connect(window, app);
    history::connect(window, app);
    recents::connect(window, app);
}

/// O painel de layouts: clique aplica; a estrela e o "×" marcam a opção (spec TL, seção 3).
fn connect_layouts(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_layout_apply(move |i| {
        let Ok(i) = usize::try_from(i) else {
            return;
        };
        a.borrow_mut().timed(
            "Aplicar a opção do painel",
            |app| app.editor.apply_option(Part::Spread, i),
            |app| {
                app.refresh_strip();
                app.refresh_current();
            },
        );
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
    presets::connect(window, app);
    let a = app.clone();
    window.on_update_restart(move || {
        let mut app = a.borrow_mut();
        if app.updates.begin_restart() {
            files::request(&mut app, Pending::Update);
        }
    });
    let a = app.clone();
    window.on_update_dismiss(move || updates::dismiss(&mut a.borrow_mut()));
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

/// Os ganchos do Velopack (spec DS, regra 10): instalar, atualizar e desinstalar chamam o
/// Lamina com `--veloapp-*`, que responde e sai sem janela. Precisa ser o primeiro passo.
fn velopack_hooks() {
    let app = velopack::VelopackApp::build();
    #[cfg(windows)]
    let app = app
        .on_after_install_fast_callback(|_| install::register())
        .on_after_update_fast_callback(|_| install::register())
        .on_before_uninstall_fast_callback(|_| install::unregister());
    let mut app = app;
    app.run();
}

fn main() -> anyhow::Result<()> {
    velopack_hooks();
    let verifying = verify::requested();
    if let Some(album) = &verifying
        && let Err(reason) = verify::check(album)
    {
        eprintln!("--verificar: {reason}");
        std::process::exit(1);
    }
    #[cfg(target_os = "linux")]
    if verifying.is_none() {
        install::linux_desktop();
    }
    let window = AppWindow::new()?;
    // Abre maximizada: numa tela pequena, as faixas de baixo ficariam fora (auditoria da Fase 4, 7).
    window.window().set_maximized(true);
    let (tx, rx) = mailbox::Mailbox::new();
    let prefs = PrefsStore::load(&config_dir().join(PREFS_FILE));
    let mut editor = Editor::with_prefs(prefs);
    editor.set_strips(StripsStore::load(&config_dir().join(STRIPS_FILE)));
    editor.set_recents(RecentsStore::load(&config_dir().join(RECENTS_FILE)));
    editor.set_log(help::error_log());
    let app: Shared = Rc::new(RefCell::new(App::new(&window, Cache::system(), editor, tx)));
    // Ligada por `LAMINA_TEMPOS=1`; o temporizador precisa viver até a janela fechar.
    let (clock, _tick) = timings::Clock::install(window.window());
    {
        let mut a = app.borrow_mut();
        a.clock = clock;
        a.presets = PresetsStore::load(&config_dir().join(PRESETS_FILE));
        let models = ModelsStore::load(&config_dir().join(MODELS_FILE));
        a.editor.set_models(models);
        if let Some(warning) = a.presets.warning.take() {
            a.notice = warning;
        }
        a.updates = updates::state();
        help::start(&a);
        updates::show(&a);
    }
    connect(&window, &app);
    app.borrow_mut().refresh_all();
    recents::refresh(&app.borrow());
    recents::check(&app.borrow());
    // `lamina-app projeto.lamina` abre o projeto direto: o duplo clique no arquivo e a volta
    // depois de atualizar (spec DS, regra 5).
    let file = match &verifying {
        Some(album) => Some(album.clone().into_os_string()),
        None => std::env::args_os().nth(1),
    };
    if let Some(file) = file {
        files::open_file(&mut app.borrow_mut(), std::path::Path::new(&file));
    }
    let a = app.clone();
    if verifying.is_some() {
        Timer::single_shot(verify::SHOW_FOR, move || {
            // Fechando: um erro aqui só deixa a janela aberta até o tempo limite do CI.
            let _ = slint::quit_event_loop();
        });
    } else {
        Timer::single_shot(updates::CHECK_DELAY, move || {
            updates::on_open(&mut a.borrow_mut());
        });
    }

    // As mensagens acordam a janela na hora; o temporizador só pega as que chegaram com o estado
    // ocupado.
    mailbox::install(rx, &app);
    let poll = Timer::default();
    poll.start(
        TimerMode::Repeated,
        Duration::from_millis(250),
        mailbox::drain,
    );
    window.invoke_focus_main();
    window.run()?;
    // Sair do Lamina guarda a lâmina em que o fotógrafo parou (spec TI, regra 3).
    app.borrow_mut().editor.remember_current();
    updates::on_exit(&app.borrow());
    Ok(())
}
