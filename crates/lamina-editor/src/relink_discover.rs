//! Descobrir o que aconteceu com as fotos que sumiram com o álbum aberto (spec RF, regra 11): a
//! pasta renomeada ou movida para perto, ou as fotos renomeadas na própria pasta. Só os candidatos
//! são lidos; achou, a janela diz o que houve e "Usar o lugar novo" aplica numa edição só.

use std::path::{Path, PathBuf};

use lamina_core::{PhotoId, Project};

use crate::relink::Found;
use crate::relink_folders::{MissingFolder, missing_folders};
use crate::relink_moved::{renamed_in, same_place};

/// O que aconteceu com uma pasta ausente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discovery {
    /// A pasta ganhou outro nome, na mesma pasta de cima.
    Renamed {
        folder: MissingFolder,
        to: PathBuf,
        found: Vec<Found>,
    },
    /// A pasta foi para outro lugar perto (dentro da pasta do álbum).
    Moved {
        folder: MissingFolder,
        to: PathBuf,
        found: Vec<Found>,
    },
    /// A pasta continua lá, mas fotos dela ganharam outro nome.
    FilesRenamed {
        folder: MissingFolder,
        found: Vec<Found>,
    },
}

impl Discovery {
    /// A pasta ausente de que se trata.
    pub fn folder(&self) -> &MissingFolder {
        match self {
            Discovery::Renamed { folder, .. }
            | Discovery::Moved { folder, .. }
            | Discovery::FilesRenamed { folder, .. } => folder,
        }
    }

    pub fn found(&self) -> &[Found] {
        match self {
            Discovery::Renamed { found, .. }
            | Discovery::Moved { found, .. }
            | Discovery::FilesRenamed { found, .. } => found,
        }
    }

    /// A frase da janela: "A pasta “Casamento” foi renomeada para “Casamento Ana”." etc.
    pub fn sentence(&self) -> String {
        match self {
            Discovery::Renamed { folder, to, .. } => format!(
                "A pasta “{}” foi renomeada para “{}”.",
                folder.name(),
                to.file_name().map_or_else(
                    || to.display().to_string(),
                    |n| n.to_string_lossy().into_owned()
                )
            ),
            Discovery::Moved { folder, to, .. } => {
                format!(
                    "A pasta “{}” foi movida para {}.",
                    folder.name(),
                    to.display()
                )
            }
            Discovery::FilesRenamed { folder, found } if found.len() == 1 => {
                format!("1 foto de “{}” foi renomeada.", folder.name())
            }
            Discovery::FilesRenamed { folder, found } => {
                format!(
                    "{} fotos de “{}” foram renomeadas.",
                    found.len(),
                    folder.name()
                )
            }
        }
    }
}

/// O que aconteceu com cada pasta das fotos `missing`, quando dá para descobrir sem o fotógrafo.
/// `album` é o arquivo do álbum (a pasta dele também é procurada). Lê pastas e arquivos: a janela
/// chama fora da linha dela.
pub fn discover(project: &Project, missing: &[PhotoId], album: Option<&Path>) -> Vec<Discovery> {
    missing_folders(project, missing)
        .into_iter()
        .filter(|f| f.disk.is_none())
        .filter_map(|folder| {
            if folder.path.is_dir() {
                let found = renamed_in(project, missing, &folder.path);
                return (!found.is_empty()).then_some(Discovery::FilesRenamed { folder, found });
            }
            let parent = folder.path.parent().map(Path::to_path_buf);
            let near = parent.iter().flat_map(|p| subfolders(p)).map(|c| (c, true));
            let in_album = album
                .and_then(Path::parent)
                .into_iter()
                .flat_map(|a| std::iter::once(a.to_path_buf()).chain(subfolders(a)))
                .map(|c| (c, false));
            near.chain(in_album)
                .filter(|(c, _)| *c != folder.path)
                .map(|(c, sibling)| (same_place(project, missing, &folder.path, &c), c, sibling))
                .filter(|(found, _, _)| !found.is_empty())
                .max_by_key(|(found, _, _)| found.len())
                .map(|(found, to, sibling)| {
                    if sibling {
                        Discovery::Renamed { folder, to, found }
                    } else {
                        Discovery::Moved { folder, to, found }
                    }
                })
        })
        .collect()
}

/// As pastas logo dentro de `dir`, em ordem de nome.
fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}
