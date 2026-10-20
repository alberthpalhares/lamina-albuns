//! Fotos entrando num álbum aberto (spec IM, parte de importar): a importação de uma lista de
//! arquivos e pastas, fora da linha da janela, e as fotos novas no projeto numa edição só, em
//! ordem de captura e sem repetir (regras 1 a 5).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use lamina_core::Project;
use lamina_media::{CatalogEntry, DerivedDirs, gather_photos, import_files};

pub use lamina_media::ImportReport;

use crate::{Cache, Editor, EditorError};

/// Mensagens da seção 6.
pub const NO_PHOTOS_DROPPED: &str = "Nenhuma foto entre os arquivos soltos.";
pub const ALREADY_IN: &str = "Essas fotos já estavam no álbum.";
pub const WAIT_IMPORT: &str = "Espere a importação terminar.";
/// O motivo que a importação dá aos formatos que o Lamina ainda não lê (HEIC).
const UNSUPPORTED: &str = "formato ainda não suportado";

/// O que a importação fez, para o resumo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub added: usize,
    /// Já estavam no álbum (a mesma impressão digital).
    pub repeated: usize,
    pub unsupported: usize,
    pub not_photos: usize,
    /// Nome e motivo das fotos que não puderam ser lidas.
    pub unreadable: Vec<(String, String)>,
}

/// Importa as fotos de `paths` (pastas com as subpastas), com miniatura e imagem de tela, pelo
/// mesmo catálogo do Novo álbum. Roda fora da linha da janela.
pub fn import_photos(
    paths: &[PathBuf],
    cache: &Cache,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<ImportReport, EditorError> {
    let catalog = cache.open_catalog()?;
    let (thumbs, screens) = (cache.thumbs_dir(), cache.screens_dir());
    let dirs = DerivedDirs {
        thumbs: &thumbs,
        screens: Some(&screens),
    };
    Ok(import_files(paths, &catalog, dirs, progress)?)
}

/// Quantas fotos há em `paths` (pastas com as subpastas), sem ler nenhuma: o Novo projeto com
/// fotos soltas mostra o número antes de importar (regra 6).
pub fn photos_in(paths: &[PathBuf]) -> usize {
    gather_photos(paths).map_or(0, |g| g.photos.len())
}

/// Põe no projeto as fotos novas do relatório e deixa todas em ordem de captura. As que já
/// estavam (mesma impressão digital) só entram na contagem.
pub fn add_imported(project: &mut Project, report: &ImportReport) -> ImportSummary {
    let fresh = fresh_entries(project, report);
    let mut summary = summary_of(report);
    summary.added = fresh.len();
    summary.repeated = report.entries.len() - fresh.len();
    for entry in fresh {
        push_entry(project, entry);
    }
    project.photos.sort_by(|a, b| {
        (a.taken_at.is_none(), a.taken_at, &a.path).cmp(&(
            b.taken_at.is_none(),
            b.taken_at,
            &b.path,
        ))
    });
    summary
}

/// As entradas do relatório que ainda não estão no projeto, sem repetir entre si.
fn fresh_entries<'a>(project: &Project, report: &'a ImportReport) -> Vec<&'a CatalogEntry> {
    let mut known: HashSet<&str> = project
        .photos
        .iter()
        .filter_map(|p| p.fingerprint.as_deref())
        .collect();
    report
        .entries
        .iter()
        .filter(|e| known.insert(e.fingerprint.as_str()))
        .collect()
}

/// A foto do catálogo no projeto, como o Novo álbum faz.
pub(crate) fn push_entry(project: &mut Project, entry: &CatalogEntry) {
    project.add_photo(&entry.path, entry.width, entry.height);
    if let Some(photo) = project.photos.last_mut() {
        photo.taken_at = entry.taken_at;
        photo.camera.clone_from(&entry.camera);
        photo.fingerprint = Some(entry.fingerprint.clone());
    }
}

fn summary_of(report: &ImportReport) -> ImportSummary {
    let (unsupported, unreadable): (Vec<_>, Vec<_>) = report
        .failures
        .iter()
        .partition(|(_, why)| why == UNSUPPORTED);
    ImportSummary {
        unsupported: unsupported.len(),
        not_photos: report.not_photos,
        unreadable: unreadable
            .into_iter()
            .map(|(path, why)| (file_name(path), why.clone()))
            .collect(),
        ..ImportSummary::default()
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

impl ImportSummary {
    /// "24 fotos importadas · 3 já estavam no álbum · 1 em formato não suportado · 2 arquivos não
    /// são fotos · 1 foto não pôde ser lida: x.jpg (motivo)".
    pub fn text(&self) -> String {
        if self.added == 0 && self.repeated > 0 && self.is_clean() {
            return ALREADY_IN.to_owned();
        }
        if self.added == 0 && self.repeated == 0 && self.is_clean() {
            return NO_PHOTOS_DROPPED.to_owned();
        }
        let n = |count: usize, one: &str, many: &str| {
            format!("{count} {}", if count == 1 { one } else { many })
        };
        let mut parts = Vec::new();
        if self.added > 0 {
            parts.push(n(self.added, "foto importada", "fotos importadas"));
        }
        if self.repeated > 0 {
            parts.push(n(
                self.repeated,
                "já estava no álbum",
                "já estavam no álbum",
            ));
        }
        if self.unsupported > 0 {
            parts.push(format!("{} em formato não suportado", self.unsupported));
        }
        if self.not_photos > 0 {
            parts.push(n(
                self.not_photos,
                "arquivo não é foto",
                "arquivos não são fotos",
            ));
        }
        if !self.unreadable.is_empty() {
            parts.push(self.unreadable_text());
        }
        parts.join(" · ")
    }

    fn is_clean(&self) -> bool {
        self.unsupported == 0 && self.not_photos == 0 && self.unreadable.is_empty()
    }

    fn unreadable_text(&self) -> String {
        match self.unreadable.as_slice() {
            [(name, why)] => format!("1 foto não pôde ser lida: {name} ({why})"),
            many => {
                let names: Vec<&str> = many.iter().map(|(n, _)| n.as_str()).collect();
                format!(
                    "{} fotos não puderam ser lidas: {}",
                    many.len(),
                    names.join(", ")
                )
            }
        }
    }
}

impl Editor {
    /// As fotos importadas no álbum aberto, numa edição só ("Importar 24 fotos"), que Ctrl+Z
    /// desfaz. Sem foto nova, o histórico não muda (IM-B13).
    pub fn add_photos(&mut self, report: &ImportReport) -> Result<ImportSummary, EditorError> {
        let session = self.session_mut().ok_or(EditorError::NoProject)?;
        let fresh = fresh_entries(session.project(), report).len();
        if fresh == 0 {
            let mut summary = summary_of(report);
            summary.repeated = report.entries.len();
            return Ok(summary);
        }
        let word = if fresh == 1 { "foto" } else { "fotos" };
        let mut summary = ImportSummary::default();
        session.edit(&format!("Importar {fresh} {word} · álbum"), |p| {
            summary = add_imported(p, report);
            Ok::<(), EditorError>(())
        })?;
        Ok(summary)
    }
}
