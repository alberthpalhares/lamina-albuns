//! `lamina-app --verificar <álbum>` (spec DS, regra 12): o teste de fumaça da release. Abre a
//! janela, carrega o álbum, desenha a primeira lâmina e fecha: código 0, ou 1 com o motivo.

use std::path::PathBuf;

use lamina_editor::{EXAMPLE_DIR, EXAMPLE_FILE, Editor, EditorError, Human};

/// Tempo com a janela aberta, para a primeira lâmina chegar à tela.
pub const SHOW_FOR: std::time::Duration = std::time::Duration::from_secs(3);

/// O álbum de `--verificar <álbum>`, se é essa a chamada. `--verificar exemplo` é o álbum de
/// exemplo que veio com o programa (spec PC, regra 8).
pub fn requested() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    if args.next()? != "--verificar" {
        return None;
    }
    let album = args.next()?;
    Some(if album == EXAMPLE_DIR {
        crate::welcome::example_source().join(EXAMPLE_FILE)
    } else {
        PathBuf::from(album)
    })
}

/// Confere o álbum antes de abrir a janela: um arquivo que não é álbum sai com 1 sem janela.
pub fn check(path: &std::path::Path) -> Result<(), String> {
    // O exemplo que não veio com o programa tem a frase própria (spec PC, PC-B8).
    if !path.is_file() && path.file_name().is_some_and(|n| n == EXAMPLE_FILE) {
        let folder = path.parent().unwrap_or(path).display().to_string();
        return Err(EditorError::NoExample(folder).human().line());
    }
    let mut editor = Editor::new();
    editor.open(path).map_err(|e| {
        let message = e.human();
        format!("{}\n{}", message.line(), message.details)
    })?;
    if editor.session().is_some_and(|s| s.spread_count() > 0) {
        Ok(())
    } else {
        Err("o álbum não tem lâminas".to_owned())
    }
}
