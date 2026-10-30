//! Reencontrar fotos (spec EI, regras 5 a 7): sozinho ao abrir, fora da linha da janela, e o
//! resultado de toda procura. A janela das pastas ausentes mora em `relink_dialog` (spec RF).

use std::path::PathBuf;

use lamina_editor::{
    Found, HumanMessage, RELINK_LATER, guarded, not_found_message, relinked_message, search_near,
};

use crate::files::show_error;
use crate::state::{App, Msg};

/// Ao abrir com fotos ausentes: procura perto do projeto e no catálogo, sem travar a janela.
pub fn near(app: &App) {
    let Some(session) = app.session() else {
        return;
    };
    if session.missing().is_empty() {
        return;
    }
    let project = session.project_shared();
    let missing = session.missing().to_vec();
    let file = session.file().map(std::path::Path::to_path_buf);
    let (cache, tx) = (app.cache.clone(), app.tx.clone());
    std::thread::spawn(move || {
        let found = guarded(|| search_near(&project, &missing, file.as_deref(), &cache));
        // Se a janela já fechou, não há a quem avisar.
        let _ = tx.send(match found {
            Ok(found) => Msg::Relinked {
                file,
                found,
                folder: None,
            },
            Err(problem) => Msg::Failed(problem),
        });
    });
}

/// Andamento e resultado da procura: as fotos achadas entram como uma edição.
pub fn message(app: &mut App, msg: Msg) {
    match msg {
        Msg::Searching(done, total) => {
            if let Some(w) = app.window.upgrade() {
                w.set_busy_text(format!("Procurando as fotos… {done} de {total}").into());
            }
        }
        Msg::Relinked {
            file,
            found,
            folder,
        } => relinked(app, file, &found, folder),
        _ => {}
    }
}

fn relinked(app: &mut App, file: Option<PathBuf>, found: &[Found], folder: Option<PathBuf>) {
    // Só a procura numa pasta escolhida trava a janela; a de abrir corre sem andamento (e não
    // pode apagar o de uma exportação em curso).
    if folder.is_some() {
        app.editor.set_busy(false);
        if let Some(w) = app.window.upgrade() {
            w.set_busy_text("".into());
            w.invoke_focus_main();
        }
    }
    // Outro projeto foi aberto enquanto a procura corria: o resultado não é dele.
    let Some(session) = app.editor.session_mut() else {
        return;
    };
    if session.file().map(std::path::Path::to_path_buf) != file {
        return;
    }
    let before = session.missing().len();
    let changed = if folder.is_some() {
        session.relink(found)
    } else {
        match session.relink_after_open(found) {
            Some(n) => n,
            None => {
                if !found.is_empty() {
                    app.notice = RELINK_LATER.to_owned();
                    app.refresh_current();
                }
                crate::relink_dialog::after_open(app);
                return;
            }
        }
    };
    let still = session.missing().len();
    if changed > 0 {
        // As fotos achadas precisam das imagens de tela: recarrega como um projeto novo, mas
        // na mesma lâmina.
        app.project_changed();
        app.notice = relinked_message(changed, still).unwrap_or_default();
        app.refresh_current();
    }
    match folder {
        // A janela das pastas ausentes mostra o resultado dentro dela (spec RF, seção 6).
        Some(folder) if crate::relink_dialog::searched(app, changed, &folder) => {}
        Some(folder) if changed == 0 => {
            let message = not_found_message(before, &folder);
            show_error(
                app,
                &HumanMessage::new("Nenhuma foto ausente foi encontrada.", message, ""),
            );
        }
        Some(_) => {}
        // Depois da procura de sempre ao abrir (spec RF, regra 2).
        None => crate::relink_dialog::after_open(app),
    }
}
