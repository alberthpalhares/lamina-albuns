//! A sessão de edição (spec JP, seção 4): o projeto aberto, o arquivo dele, a lâmina atual, as
//! alterações não salvas, as fotos ausentes e os avisos de pré-impressão.

use std::path::{Path, PathBuf};

use lamina_core::{History, PhotoId, Project};
use lamina_export::{Warning, preflight, read_project, save_project};

use crate::EditorError;
use crate::photos::used_photos;

mod edit;

/// Quantos nomes de fotos ausentes aparecem no aviso antes de "e mais N".
const MISSING_NAMES: usize = 5;

#[derive(Debug)]
pub struct Session {
    history: History<Project>,
    file: Option<PathBuf>,
    /// O projeto como está no disco; `None` = nunca salvo.
    saved: Option<Project>,
    current: usize,
    missing: Vec<PhotoId>,
    warnings: Vec<Warning>,
}

impl Session {
    /// Álbum novo, ainda sem arquivo: nasce com alterações não salvas (regra 8).
    pub fn new_unsaved(project: Project) -> Self {
        Session::with(project, None, false)
    }

    /// Abre o arquivo, mesmo com fotos ausentes (regra 11).
    pub fn open(file: &Path) -> Result<Self, EditorError> {
        let project = read_project(file)?;
        let file = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
        Ok(Session::with(project, Some(file), true))
    }

    fn with(project: Project, file: Option<PathBuf>, saved: bool) -> Self {
        let missing = project
            .photos
            .iter()
            .filter(|p| !p.path.exists())
            .map(|p| p.id)
            .collect();
        let warnings = warnings_of(&project);
        Session {
            saved: saved.then(|| project.clone()),
            history: History::new(project),
            file,
            current: 0,
            missing,
            warnings,
        }
    }

    pub fn project(&self) -> &Project {
        self.history.present()
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.saved.as_ref() != Some(self.project())
    }

    /// Nome do projeto, para o título da janela.
    pub fn title(&self) -> &str {
        &self.project().name
    }

    /// Lâmina atual, contada a partir de 0.
    pub fn current(&self) -> usize {
        self.current
    }

    pub fn spread_count(&self) -> usize {
        self.project().spreads.len()
    }

    /// Vai para a lâmina `index`; `false` (e nada muda) se ela não existe.
    pub fn go_to(&mut self, index: usize) -> bool {
        if index >= self.spread_count() {
            return false;
        }
        self.current = index;
        true
    }

    /// "3 lâminas (6 páginas) · 12 de 12 fotos usadas".
    pub fn counter(&self) -> String {
        let project = self.project();
        let spreads = project.spreads.len();
        let pages = project.spec.edges().page_count(spreads);
        let (s, p) = (plural(spreads, "lâmina"), plural(pages, "página"));
        let used = used_photos(project);
        let total = project.photos.len();
        format!("{spreads} {s} ({pages} {p}) · {used} de {total} fotos usadas")
    }

    pub fn missing(&self) -> &[PhotoId] {
        &self.missing
    }

    /// "1 foto não foi encontrada: IMG_0003.jpg." (até 5 nomes, depois "e mais N").
    pub fn missing_message(&self) -> Option<String> {
        let names: Vec<String> = self
            .missing
            .iter()
            .filter_map(|id| self.project().photo(*id).ok())
            .map(|p| file_name(&p.path))
            .collect();
        let n = names.len();
        let list = match n {
            0 => return None,
            1 => names[0].clone(),
            _ if n > MISSING_NAMES => format!(
                "{} e mais {}",
                names[..MISSING_NAMES].join(", "),
                n - MISSING_NAMES
            ),
            _ => format!("{} e {}", names[..n - 1].join(", "), names[n - 1]),
        };
        Some(if n == 1 {
            format!("1 foto não foi encontrada: {list}.")
        } else {
            format!("{n} fotos não foram encontradas: {list}.")
        })
    }

    /// Avisos de pré-impressão da lâmina `index` (contada a partir de 0).
    pub fn warnings_for(&self, index: usize) -> Vec<&Warning> {
        self.warnings
            .iter()
            .filter(|w| w.spread == index + 1)
            .collect()
    }

    /// Salva no arquivo da sessão.
    pub fn save(&mut self) -> Result<PathBuf, EditorError> {
        let file = self.file.clone().ok_or(EditorError::NoFile)?;
        self.save_as(&file)
    }

    /// Salva em `file` e passa a usar esse arquivo (regra 9).
    pub fn save_as(&mut self, file: &Path) -> Result<PathBuf, EditorError> {
        let file = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
        let folder = file.parent().unwrap_or(Path::new("."));
        let name = file_name(&file);
        let saved = save_project(self.project(), folder, &name)?;
        self.saved = Some(self.project().clone());
        self.file = Some(saved.clone());
        Ok(saved)
    }
}

/// Avisos de pré-impressão de todas as lâminas.
fn warnings_of(project: &Project) -> Vec<Warning> {
    let all: Vec<usize> = (0..project.spreads.len()).collect();
    preflight(project, &all)
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        word.to_owned()
    } else {
        format!("{word}s")
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}
