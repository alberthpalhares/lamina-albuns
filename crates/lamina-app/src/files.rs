//! Abrir, salvar e a pergunta antes de sair (spec JP, regras 8 e 9), com os diálogos de arquivo
//! do sistema (`rfd`).

use std::path::{Path, PathBuf};

use lamina_editor::{Human, HumanMessage, Leave};
use slint::ComponentHandle;

use crate::state::{App, Pending};

const FILTER_NAME: &str = "Álbum do Lamina";
const EXT: &str = "lamina";

/// O diálogo de erro com a mensagem humana: a frase, o que fazer e o detalhe em "Mostrar
/// detalhes" (spec EH). Toda mensagem mostrada vai para o registro (spec SR, regra 5).
pub fn show_error(app: &App, message: &HumanMessage) {
    app.editor.record(message);
    if let Some(w) = app.window.upgrade() {
        w.set_message_title(message.title.as_str().into());
        w.set_message_text(message.what_to_do.as_str().into());
        w.set_message_details(message.details.as_str().into());
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
    // Salvar põe o álbum no topo dos recentes e gera a miniatura (spec TI, regras 1 e 5).
    let result = app.editor.save_to(target.as_deref());
    app.refresh_current();
    match result {
        Ok(_) => {
            crate::recents::thumbnail(app);
            crate::recents::refresh(app);
            true
        }
        Err(e) => {
            show_error(app, &e.human());
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
    } else if action == Pending::Update {
        crate::updates::restart_cancelled(app);
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
        Pending::NewAlbum => crate::new_project::start(app),
        Pending::Update => crate::updates::restart(app),
        Pending::Example => crate::welcome::open_example(app),
        Pending::OpenRecent(i) => crate::recents::open(app, i),
        Pending::CloseAlbum => {
            if app.editor.close_album() {
                app.project_changed();
                crate::recents::refresh(app);
                crate::recents::check(app);
            }
        }
    }
}

fn open(app: &mut App) {
    let Some(file) = rfd::FileDialog::new()
        .set_title("Abrir álbum")
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
            crate::recents::refresh(app);
        }
        Err(e) => show_error(app, &e.human()),
    }
}
