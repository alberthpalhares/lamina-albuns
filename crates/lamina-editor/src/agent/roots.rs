//! Pastas permitidas (spec IA, regra 5): a IA só lê pastas novas e só grava dentro delas. O
//! caminho é conferido depois de resolver `..` e atalhos, para "A\..\C" não escapar de A.

use std::path::{Component, Path, PathBuf};

use super::ToolError;

#[derive(Debug, Clone, Default)]
pub struct AllowedRoots {
    roots: Vec<PathBuf>,
}

impl AllowedRoots {
    pub fn new(folders: &[PathBuf]) -> Self {
        let mut roots = AllowedRoots::default();
        for folder in folders {
            roots.add(folder);
        }
        roots
    }

    /// Passa a permitir `folder` (e o que houver dentro dela).
    pub fn add(&mut self, folder: &Path) {
        let resolved = resolve(folder);
        if !self.roots.contains(&resolved) {
            self.roots.push(resolved);
        }
    }

    /// As pastas, já resolvidas, na ordem em que entraram.
    pub fn list(&self) -> &[PathBuf] {
        &self.roots
    }

    /// `path` está dentro de uma pasta permitida? Devolve o caminho absoluto, sem `..`.
    pub fn check(&self, path: &Path) -> Result<PathBuf, ToolError> {
        let resolved = resolve(path);
        if self.roots.iter().any(|root| resolved.starts_with(root)) {
            Ok(lexical(path))
        } else {
            Err(ToolError::NotAllowed(folder_of(path).display().to_string()))
        }
    }
}

/// O caminho absoluto com `.` e `..` resolvidos pelo texto, sem tocar o disco.
fn lexical(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// O caminho resolvido: o maior pedaço que existe passa por `canonicalize` (atalhos e
/// maiúsculas do disco); o resto, que ainda não existe, vai junto pelo texto.
fn resolve(path: &Path) -> PathBuf {
    let plain = lexical(path);
    let mut existing = plain.as_path();
    let mut rest = Vec::new();
    loop {
        if let Ok(real) = existing.canonicalize() {
            return rest.iter().rev().fold(real, |acc, part| acc.join(part));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => return plain,
        }
    }
}

/// A pasta citada na recusa: o próprio caminho, se for pasta; senão, a pasta dele.
fn folder_of(path: &Path) -> PathBuf {
    let plain = lexical(path);
    if plain.is_dir() || plain.extension().is_none() {
        plain
    } else {
        plain
            .parent()
            .map_or_else(|| plain.clone(), Path::to_path_buf)
    }
}
