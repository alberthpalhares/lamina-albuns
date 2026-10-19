//! Caminhos de arquivo das ferramentas: a extensão `.lamina`, a pasta do arquivo e os nomes nas
//! respostas.

use std::path::{Path, PathBuf};

use super::ToolError;

/// Cria a pasta do arquivo, que já passou pelas pastas permitidas.
pub(super) fn ensure_parent(file: &Path) -> Result<(), ToolError> {
    let Some(folder) = file.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(folder).map_err(|e| {
        ToolError::Refused(format!(
            "Não foi possível criar a pasta {}: {e}.",
            folder.display()
        ))
    })
}

/// O caminho com a extensão `.lamina`, que a IA pode ter esquecido (como o "Salvar como").
pub(super) fn lamina_file(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    let ext = lamina_export::PROJECT_EXT;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    {
        return path;
    }
    let mut name = path.into_os_string();
    name.push(format!(".{ext}"));
    PathBuf::from(name)
}

/// Os dois caminhos são o mesmo arquivo?
pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// O nome do arquivo, sem a pasta.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}
