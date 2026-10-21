//! Interface gráfica do Lamina (spec `janela-principal`; ADR 0002).
//!
//! Este binário só traduz (regra 1): as telas estão em `ui/*.slint` e toda decisão vem do
//! `lamina-editor`. O trabalho longo roda em outras linhas e volta por um canal, lido a cada
//! 50 ms pela linha da janela.

#![windows_subsystem = "windows"]

mod adjusted;
mod browser;
mod calibrate;
mod context;
mod convert;
mod crop;
mod details;
mod drag_motion;
mod drops;
mod export;
mod files;
mod frame_panel;
mod ghost;
mod hand;
mod help;
mod history;
mod images;
mod importing;
mod input;
mod install;
mod line_window;
mod lists;
mod loader;
mod look;
mod mailbox;
mod models;
mod monitor;
mod motion;
mod msg;
mod new_album;
mod new_project;
mod new_project_file;
mod news;
mod news_player;
mod opened;
mod organize;
mod palette;
mod panel;
mod photo_panel;
mod presets;
mod recents;
mod refresh;
mod relink;
mod roll;
mod roll_cells;
mod roll_connect;
mod roll_stats;
mod roll_worker;
mod shortcuts;
mod slots;
mod spec_dialog;
mod state;
mod texts;
mod thumb_loader;
mod thumbs;
mod timings;
mod updates;
mod verify;
mod welcome;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_editor::{
    Cache, Command, Editor, Install, MODELS_FILE, ModelsStore, PREFS_FILE, PRESETS_FILE, Part,
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
    shortcuts::connect(window);
    news::connect(window);
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
    browser::connect(window, app);
    connect_dialogs(window, app);
    connect_layouts(window, app);
    drops::connect(window, app);
    drag_motion::connect_motion(window, app);
    organize::connect(window, app);
    context::connect(window, app);
    palette::connect(window, app);
    photo_panel::connect(window, app);
    crop::connect(window, app);
    look::connect(window, app);
    hand::connect(window, app);
    frame_panel::connect(window, app);
    models::connect(window, app);
    history::connect(window, app);
    recents::connect(window, app);
    importing::connect(window, app);
    calibrate::connect(window, app);
    roll_connect::connect(window, app);
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

fn connect_dialogs(window: &AppWindow, app: &Shared) {
    new_project::connect(window, app);
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
    let importing = verify::import_requested();
    if let Some(folder) = &importing
        && let Err(reason) = verify::check_import(folder)
    {
        eprintln!("--verificar-importacao: {reason}");
        std::process::exit(1);
    }
    // A verificação da abertura (spec MA): sem o relatório, sai com 1 antes da janela.
    let opening = opened::requested();
    #[cfg(target_os = "linux")]
    if verifying.is_none() {
        install::linux_desktop();
    }
    // Medir na máquina compartilhada (`LAMINA_SEM_FOCO=1`, portão G1 da spec RL): a janela abre
    // sem tomar o foco, e o teclado de quem está trabalhando em outra janela continua lá.
    if std::env::var("LAMINA_SEM_FOCO").is_ok_and(|v| v == "1") {
        slint::BackendSelector::new()
            .with_winit_window_attributes_hook(|a| a.with_active(false))
            .select()?;
    }
    let window = AppWindow::new()?;
    // Abre maximizada: numa tela pequena, as faixas de baixo ficariam fora (auditoria da Fase 4, 7).
    window.window().set_maximized(true);
    let (tx, rx) = mailbox::Mailbox::new();
    let prefs = PrefsStore::load(&config_dir().join(PREFS_FILE));
    let mut editor = Editor::with_prefs(prefs);
    // `atualizacoes.json` antes de `janela.json`: sem ele, é uma instalação nova e o painel da
    // direita começa escondido (spec JE, regras 2 e 3).
    let opened = updates::state();
    let install = Install::from_last_version(opened.store.prefs.last_version.as_deref());
    editor.set_strips(StripsStore::load_for(
        &config_dir().join(STRIPS_FILE),
        install,
    ));
    editor.set_recents(RecentsStore::load(&config_dir().join(RECENTS_FILE)));
    editor.set_log(help::error_log());
    let app: Shared = Rc::new(RefCell::new(App::new(&window, Cache::system(), editor, tx)));
    // Ligada por `LAMINA_TEMPOS=1`; o temporizador precisa viver até a janela fechar.
    let roll_on = app.borrow().roll.on;
    let (clock, _tick) = timings::Clock::install(window.window(), roll_on);
    {
        let mut a = app.borrow_mut();
        a.clock = clock;
        a.presets = PresetsStore::load(&config_dir().join(PRESETS_FILE));
        let models = ModelsStore::load(&config_dir().join(MODELS_FILE));
        a.editor.set_models(models);
        if let Some(warning) = a.presets.warning.take() {
            a.notice = warning;
        }
        a.updates = opened;
        help::start(&a);
        // As novidades da versão nova, uma vez, antes de a janela aparecer (spec NV, regra 3); o
        // teste de fumaça não é interrompido.
        if verifying.is_none() && importing.is_none() {
            news::on_start(&mut a);
        }
        updates::show(&a);
    }
    connect(&window, &app);
    app.borrow_mut().refresh_all();
    recents::refresh(&app.borrow());
    recents::check(&app.borrow());
    // `lamina-app projeto.lamina` abre o projeto direto: o duplo clique no Windows e no Linux e a
    // volta depois de atualizar (spec DS, regra 5; spec MA, regra 9). No Mac, chega por evento.
    if let Some(album) = &verifying {
        files::open_file(&mut app.borrow_mut(), album);
    } else if importing.is_none() && opening.is_none() {
        opened::from_args(&mut app.borrow_mut());
    }
    // O teste de fumaça da importação (spec TR): fecha sozinho quando as fotos entram.
    if let Some(folder) = importing.clone()
        && let Err(reason) = verify::start_import(&mut app.borrow_mut(), folder)
    {
        eprintln!("--verificar-importacao: {reason}");
        std::process::exit(1);
    }
    let a = app.clone();
    if verifying.is_some() {
        Timer::single_shot(verify::SHOW_FOR, move || {
            // Fechando: um erro aqui só deixa a janela aberta até o tempo limite do CI.
            let _ = slint::quit_event_loop();
        });
    } else if importing.is_none() && opening.is_none() {
        // Os testes de fumaça não procuram atualização: não dependem da rede.
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
    // Os arquivos que o sistema entrega, com o Lamina aberto ou abrindo (spec MA).
    let _listener = opened::listen(&app, opening);
    window.invoke_focus_main();
    window.run()?;
    // Sair do Lamina guarda a lâmina em que o fotógrafo parou (spec TI, regra 3).
    app.borrow_mut().editor.remember_current();
    updates::on_exit(&app.borrow());
    Ok(())
}
