//! `lamina-app --verificar <álbum>` (spec DS, regra 12): o teste de fumaça da release. Abre a
//! janela, carrega o álbum, desenha a primeira lâmina e fecha: código 0, ou 1 com o motivo.

use std::path::PathBuf;

use lamina_editor::Editor;

/// Tempo com a janela aberta, para a primeira lâmina chegar à tela.
pub const SHOW_FOR: std::time::Duration = std::time::Duration::from_secs(3);

/// O álbum de `--verificar <álbum>`, se é essa a chamada.
pub fn requested() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    (args.next()? == "--verificar").then(|| args.next().map(PathBuf::from))?
}

/// Confere o álbum antes de abrir a janela: um arquivo que não é álbum sai com 1 sem janela.
pub fn check(path: &std::path::Path) -> Result<(), String> {
    let mut editor = Editor::new();
    editor.open(path).map_err(|e| e.to_string())?;
    if editor.session().is_some_and(|s| s.spread_count() > 0) {
        Ok(())
    } else {
        Err("o álbum não tem lâminas".to_owned())
    }
}
