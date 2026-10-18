//! Abrir, salvar e a pergunta antes de sair (spec JP, regras 8 e 9), com os diálogos de arquivo
//! do sistema (`rfd`).

use std::path::{Path, PathBuf};

use lamina_editor::Leave;
use slint::ComponentHandle;

use crate::new_album;
use crate::state::{App, Pending};

const FILTER_NAME: &str = "Projeto do Lamina";
const EXT: &str = "lamina";

pub fn show_error(app: &App, title: &str, message: &str) {
    if let Some(w) = app.window.upgrade() {
        w.set_message_title(title.into());
        w.set_message_text(message.into());
        w.set_message_folder("".into());
        w.set_show_message(true);
    }
}

/// "Salvar como": o arquivo escolhido, sempre com a extensão `.lamina`.
fn ask_save_path(app: &App) -> Option<PathBuf> {
    let name = app.session()?.title().to_owned();
    let path = rfd::FileDialog::new()
        .set_title("Salvar como")
        .add_filter(FILTER_NAME, &[EXT])
        .set_file_name(format!("{name}.{EXT}"))
        .save_file()?;
    Some(if path.extension().is_some_and(|e| e == EXT) {
        path
    } else {
        path.with_extension(EXT)
    })
}

/// Salva (ou "Salvar como", se `as_new` ou se ainda não tem arquivo). `true` = salvou.
pub fn save(app: &mut App, as_new: bool) -> bool {
    let Some(session) = app.editor.session() else {
        return false;
    };
    let target = if as_new || session.file().is_none() {
        match ask_save_path(app) {
            Some(path) => Some(path),
            None => return false,
        }
    } else {
        None
    };
    let Some(session) = app.editor.session_mut() else {
        return false;
    };
    let result = match &target {
        Some(path) => session.save_as(path),
        None => session.save(),
    };
    app.refresh_current();
    match result {
        Ok(_) => true,
        Err(e) => {
            show_error(app, "Não foi possível salvar", &e.to_string());
            false
        }
    }
}

/// Fechar, abrir ou criar: pergunta antes se há alterações não salvas.
pub fn request(app: &mut App, action: Pending) {
    if app.editor.needs_confirmation() {
        app.pending = Some(action);
        if let Some(w) = app.window.upgrade() {
            w.set_show_confirm(true);
        }
    } else {
        proceed(app, action);
    }
}

/// Resposta à pergunta: 0 = Salvar, 1 = Não salvar, 2 = Cancelar.
pub fn answer(app: &mut App, choice: i32) {
    if let Some(w) = app.window.upgrade() {
        w.set_show_confirm(false);
        w.invoke_focus_main();
    }
    let Some(action) = app.pending.take() else {
        return;
    };
    let choice = match choice {
        0 => Leave::Save,
        1 => Leave::DontSave,
        _ => Leave::Cancel,
    };
    let go = if choice == Leave::Save {
        save(app, false)
    } else {
        app.editor.leave(choice, None).unwrap_or(false)
    };
    if go {
        proceed(app, action);
    }
}

fn proceed(app: &mut App, action: Pending) {
    match action {
        Pending::Close => {
            if let Some(w) = app.window.upgrade() {
                // Já fechando: um erro aqui não muda nada.
                let _ = w.hide();
            }
        }
        Pending::Open => open(app),
        Pending::NewAlbum => new_album::start(app),
    }
}

fn open(app: &mut App) {
    let Some(file) = rfd::FileDialog::new()
        .set_title("Abrir projeto")
        .add_filter(FILTER_NAME, &[EXT])
        .pick_file()
    else {
        return;
    };
    open_file(app, &file);
}

/// Abre `file`; se não der, mostra o erro e o projeto anterior continua aberto.
pub fn open_file(app: &mut App, file: &Path) {
    match app.editor.open(file) {
        Ok(()) => {
            app.project_changed();
            crate::relink::near(app);
        }
        Err(e) => show_error(app, "Não foi possível abrir o projeto", &e.to_string()),
    }
}
