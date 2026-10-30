//! O menu de botão direito da grade de fotos e o Delete com a grade em foco (spec TF): tirar do
//! álbum, com a pergunta quando há lâmina afetada, e mostrar o arquivo no Explorador ou no
//! Finder. As decisões ficam no `lamina-editor` (regra 1).

use std::cell::RefCell;
use std::rc::Rc;

use lamina_core::PhotoId;
use lamina_editor::Human;
use lamina_editor::leaving::{REMOVE_BUTTON, leave_question, remove_label};
use lamina_editor::reveal::{System, reveal_command};
use slint::ComponentHandle;

use crate::state::App;
use crate::{AppWindow, PhotoMenuUi};

type Shared = Rc<RefCell<App>>;

/// Os textos do menu seguem a seleção e o sistema (regra 1).
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PhotoMenuUi>();
    ui.set_remove_text(remove_label(app.editor.selection().len().max(1)).into());
    ui.set_reveal_text(System::current().reveal_label().into());
}

/// "Remover fotografia do álbum" ou Delete na grade: pergunta só se alguma foto está em lâmina
/// (regra 4).
pub fn ask_remove(app: &mut App) {
    let photos = app.editor.selection().to_vec();
    ask_remove_these(app, photos);
}

/// Tira `photos` do álbum, perguntando antes se alguma está em lâmina; o mesmo nome e a mesma
/// pergunta pela grade e pelo menu do quadro (TF-B11 e TF-B12).
pub fn ask_remove_these(app: &mut App, photos: Vec<PhotoId>) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let used = app.editor.photos_in_spreads(&photos);
    app.leaving = photos;
    match leave_question(used) {
        None => remove(app),
        Some(question) => {
            refresh(app);
            let ui = w.global::<PhotoMenuUi>();
            ui.set_remove_text(remove_label(app.leaving.len()).into());
            ui.set_confirm_text(REMOVE_BUTTON.into());
            ui.set_question(question.into());
            ui.set_show_ask(true);
        }
    }
}

fn remove(app: &mut App) {
    let photos = std::mem::take(&mut app.leaving);
    if let Some(w) = app.window.upgrade() {
        w.global::<PhotoMenuUi>().set_show_ask(false);
    }
    if app.editor.remove_from_album(&photos) {
        app.project_changed();
    }
}

/// Abre a pasta da primeira foto selecionada, com o arquivo marcado (regra 7).
fn reveal(app: &mut App) {
    let Some(file) = app
        .editor
        .selection()
        .first()
        .and_then(|id| app.editor.photo_file(*id))
    else {
        return;
    };
    match reveal_command(&file, System::current()) {
        Ok((program, args)) => {
            // O Explorador sai com código 1 mesmo quando abre: não há o que conferir.
            let _ = command(&program, &args).spawn();
        }
        Err(e) => crate::files::show_error(app, &e.human()),
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PhotoMenuUi>();
    let on = |f: fn(&mut App)| {
        let a = app.clone();
        move || f(&mut a.borrow_mut())
    };
    ui.on_remove(on(ask_remove));
    ui.on_confirm(on(remove));
    ui.on_reveal(on(reveal));
    refresh(&app.borrow());
}

/// No Windows, os argumentos vão como estão: o `/select,"caminho"` já vem com as aspas certas, e
/// as do Rust em volta dele todo fariam o Explorador abrir outra pasta (TF-B13).
#[cfg(windows)]
fn command(program: &str, args: &[String]) -> std::process::Command {
    use std::os::windows::process::CommandExt as _;
    let mut command = std::process::Command::new(program);
    for arg in args {
        command.raw_arg(arg);
    }
    command
}

#[cfg(not(windows))]
fn command(program: &str, args: &[String]) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.args(args);
    command
}
