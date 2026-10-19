//! O álbum de exemplo (spec PC, regras 3 a 5): vem junto do programa e é copiado para a pasta de
//! configuração antes de abrir, porque a pasta do programa é trocada a cada atualização (e, no Mac
//! e no Linux, nem aceita gravação). A cópia nunca apaga nem sobrescreve.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{EditorError, config_dir};

/// A pasta do exemplo ao lado do programa (e no repositório, em `crates/lamina-app`).
pub const EXAMPLE_DIR: &str = "exemplo";
/// O álbum dentro dela.
pub const EXAMPLE_FILE: &str = "exemplo.lamina";
/// A pasta da cópia, dentro da pasta de configuração, e o nome do álbum copiado.
pub const EXAMPLE_COPY: &str = "Álbum de exemplo";

/// Onde fica a cópia do fotógrafo.
pub fn example_target() -> PathBuf {
    config_dir().join(EXAMPLE_COPY)
}

/// Copia para `target` o que falta do exemplo em `source` e devolve o álbum copiado. O álbum muda
/// de nome para "Álbum de exemplo.lamina" (o título da janela); o resto mantém o caminho, e as
/// fotos são achadas porque o `.lamina` guarda caminhos relativos (ADR 0004).
pub fn prepare_example(source: &Path, target: &Path) -> Result<PathBuf, EditorError> {
    if !source.join(EXAMPLE_FILE).is_file() {
        return Err(EditorError::NoExample(source.display().to_string()));
    }
    copy_missing(source, target)?;
    Ok(target.join(copy_name()))
}

fn copy_name() -> String {
    format!("{EXAMPLE_COPY}.lamina")
}

/// Copia cada arquivo que ainda não existe no destino, pasta por pasta.
fn copy_missing(source: &Path, target: &Path) -> Result<(), EditorError> {
    let entries = fs::read_dir(source).map_err(|e| copy_error(source, &e))?;
    for entry in entries {
        let from = entry.map_err(|e| copy_error(source, &e))?.path();
        let Some(name) = from.file_name() else {
            continue;
        };
        let to = if name == EXAMPLE_FILE {
            target.join(copy_name())
        } else {
            target.join(name)
        };
        if from.is_dir() {
            copy_missing(&from, &to)?;
        } else if !to.exists() {
            fs::create_dir_all(target).map_err(|e| copy_error(target, &e))?;
            fs::copy(&from, &to).map_err(|e| copy_error(&to, &e))?;
        }
    }
    Ok(())
}

fn copy_error(path: &Path, error: &std::io::Error) -> EditorError {
    EditorError::CopyExample {
        path: path.display().to_string(),
        reason: lamina_media::io_reason(error),
    }
}
