//! A cópia de segurança na janela (spec CS): a cada minuto, a cópia do álbum com alterações não
//! salvas, gravada fora da linha da janela; na abertura, a pergunta "Recuperar?". As decisões
//! ficam no `lamina-editor` (`backup`).

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::backup::BACKUP_EVERY;
use lamina_editor::recents::Clock;
use slint::{ComponentHandle, Timer, TimerMode};

use crate::state::App;
use crate::{AppWindow, RecoverUi};

type Shared = Rc<RefCell<App>>;

/// O passo da cópia: grava a cópia pedida (fora da linha da janela) ou apaga a que sobrou.
/// Também depois de salvar, abrir e fechar, para a cópia não durar mais que as alterações.
pub fn tick(app: &mut App) {
    if let Some(job) = app.editor.backup_step() {
        // Uma cópia que não grava (disco cheio, pasta sem permissão) não interrompe o trabalho; a
        // próxima tenta de novo depois da próxima mudança.
        std::thread::spawn(move || {
            let _ = job.write();
        });
    }
}

/// O temporizador de cada minuto (regra 1); precisa viver até a janela fechar.
pub fn start(app: &Shared) -> Timer {
    let timer = Timer::default();
    let a = Rc::downgrade(app);
    timer.start(TimerMode::Repeated, BACKUP_EVERY, move || {
        if let Some(app) = a.upgrade()
            && let Ok(mut app) = app.try_borrow_mut()
        {
            tick(&mut app);
        }
    });
    timer
}

/// Na abertura: as cópias que ficaram de uma queda, uma pergunta de cada vez (regra 4).
pub fn ask(app: &mut App) {
    app.recover = app.editor.found_backups();
    show_next(app);
}

fn show_next(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<RecoverUi>();
    match app.recover.first() {
        Some(found) => {
            ui.set_question(found.question(Clock::local()).into());
            ui.set_show(true);
        }
        None => {
            ui.set_show(false);
            w.invoke_focus_main();
        }
    }
}

/// "Recuperar" e "Descartar" (regras 5 e 6).
pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<RecoverUi>();
    let a = app.clone();
    ui.on_recover(move || {
        let mut app = a.borrow_mut();
        if app.recover.is_empty() {
            return;
        }
        let found = app.recover.remove(0);
        if app.editor.recover(&found) {
            app.project_changed();
            crate::recents::refresh(&app);
        } else {
            app.notice = "A cópia de segurança não abriu e foi descartada.".to_owned();
            app.refresh_current();
        }
        show_next(&app);
    });
    let a = app.clone();
    ui.on_discard(move || {
        let mut app = a.borrow_mut();
        if app.recover.is_empty() {
            return;
        }
        let found = app.recover.remove(0);
        app.editor.discard_backup(&found);
        show_next(&app);
    });
}
