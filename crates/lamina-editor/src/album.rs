//! Novo álbum a partir de uma pasta (spec JP, regra 3): importar → sugerir lâminas → Auto Build.
//! A janela e o `lamina build` passam por aqui, então montam o mesmo álbum.

use std::path::{Path, PathBuf};

use lamina_core::{
    AlbumEdge, AlbumSpec, CoreError, Dpi, ExportPrefs, Margins, Mm, PageLimits, Project,
};
use lamina_layout::{
    AutoBuild, BuildOptions, PHOTOS_PER_SPREAD, PlanError, PlanOptions, Preferences, Reuse,
    auto_build, suggest_spreads, validate_plan,
};
use lamina_media::{
    DerivedDirs, ImageFormat, ImportReport, MediaError, import_folder_with, list_photos,
};
use serde::{Deserialize, Serialize};

use crate::{Cache, EditorError};

/// Etapas do trabalho longo, para mostrar o andamento (JP-B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Importing { done: usize, total: usize },
    Building,
}

/// As medidas do álbum como o fotógrafo digita (cm, mm, DPI), com os padrões do `lamina build`.
/// Também é o que um preset guarda (spec PR), com nomes em português no arquivo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecFields {
    #[serde(rename = "largura_cm")]
    pub width_cm: f64,
    #[serde(rename = "altura_cm")]
    pub height_cm: f64,
    pub dpi: u32,
    #[serde(rename = "sangria_mm")]
    pub bleed_mm: f64,
    #[serde(rename = "zona_segura_mm")]
    pub safe_mm: f64,
    /// Zona segura na lombada; `None` = igual à zona segura.
    #[serde(rename = "lombada_mm", default)]
    pub spine_mm: Option<f64>,
    #[serde(rename = "inicia_com_pagina", default)]
    pub starts_single: bool,
    #[serde(rename = "termina_com_pagina", default)]
    pub ends_single: bool,
}

impl Default for SpecFields {
    fn default() -> Self {
        SpecFields {
            width_cm: 30.0,
            height_cm: 30.0,
            dpi: 300,
            bleed_mm: 3.0,
            safe_mm: 10.0,
            spine_mm: None,
            starts_single: false,
            ends_single: false,
        }
    }
}

impl SpecFields {
    /// Os campos de uma especificação pronta (a janela "Especificação do álbum"; spec PR).
    pub fn from_spec(spec: &AlbumSpec) -> Self {
        let safe = spec.safe_margins();
        SpecFields {
            width_cm: spec.page_width().get() / 10.0,
            height_cm: spec.page_height().get() / 10.0,
            dpi: spec.dpi().get(),
            bleed_mm: spec.bleed().outer.get(),
            safe_mm: safe.outer.get(),
            spine_mm: ((safe.inner.get() - safe.outer.get()).abs() > 1e-9)
                .then_some(safe.inner.get()),
            starts_single: spec.edges().start == AlbumEdge::SinglePage,
            ends_single: spec.edges().end == AlbumEdge::SinglePage,
        }
    }

    pub fn to_spec(&self) -> Result<AlbumSpec, CoreError> {
        let safe = Mm(self.safe_mm);
        let inner = Mm(self.spine_mm.unwrap_or(self.safe_mm));
        let edge = |single| {
            if single {
                AlbumEdge::SinglePage
            } else {
                AlbumEdge::FullSpread
            }
        };
        AlbumSpec::new(
            Mm::from_cm(self.width_cm),
            Mm::from_cm(self.height_cm),
            Dpi::new(self.dpi)?,
        )?
        .with_bleed(Mm(self.bleed_mm))?
        .with_safe_margins(Margins::new(safe, safe, inner))
        .map(|s| s.with_edges(edge(self.starts_single), edge(self.ends_single)))
    }
}

/// O que o fotógrafo pediu na janela "Novo álbum".
#[derive(Debug, Clone)]
pub struct NewAlbum {
    pub folder: PathBuf,
    /// `None` = nome da pasta.
    pub name: Option<String>,
    pub spec: AlbumSpec,
    /// `None` = a sugestão.
    pub spreads: Option<usize>,
    pub reuse: Reuse,
    /// Favoritos e rejeitados do fotógrafo (spec TL, regra 8); vazio = escolha de sempre.
    pub preferences: Preferences,
    /// Faixa de páginas e exportação do preset (spec PR); padrão = sem limite, JPEG em sRGB.
    pub limits: PageLimits,
    pub export: ExportPrefs,
}

/// Fotos importadas, já num projeto sem lâminas, e as que não deram para importar.
#[derive(Debug)]
pub struct Imported {
    pub project: Project,
    pub failures: Vec<(PathBuf, String)>,
    /// Fotos com perfil de cor ilegível, usadas como sRGB (spec CT, seção 6).
    pub warnings: Vec<String>,
}

/// Importa a pasta e monta o projeto, com o nome pedido ou o da pasta. Com `screens`, gera
/// também as imagens de tela (a janela usa; a linha de comando não precisa).
pub fn import_project(
    folder: &Path,
    name: Option<&str>,
    spec: AlbumSpec,
    cache: &Cache,
    screens: bool,
    progress: &(dyn Fn(Step) + Sync),
) -> Result<Imported, EditorError> {
    if !folder.is_dir() {
        return Err(EditorError::MissingFolder(folder.display().to_string()));
    }
    let catalog = cache.open_catalog()?;
    let (thumbs, screens_dir) = (cache.thumbs_dir(), cache.screens_dir());
    let dirs = DerivedDirs {
        thumbs: &thumbs,
        screens: screens.then_some(screens_dir.as_path()),
    };
    let report = import_folder_with(folder, &catalog, dirs, &|done, total| {
        progress(Step::Importing { done, total });
    })?;
    if report.entries.is_empty() {
        return Err(EditorError::NoPhotos(folder.display().to_string()));
    }
    let name = name.map_or_else(|| folder_name(folder), |n| n.trim().to_owned());
    Ok(Imported {
        project: project_from(&report, &name, spec),
        failures: report.failures,
        warnings: report.warnings,
    })
}

/// Projeto com as fotos importadas, na ordem do relatório (data, câmera e impressão digital).
pub fn project_from(report: &ImportReport, name: &str, spec: AlbumSpec) -> Project {
    let mut project = Project::new(name, spec);
    for entry in &report.entries {
        project.add_photo(&entry.path, entry.width, entry.height);
        if let Some(photo) = project.photos.last_mut() {
            photo.taken_at = entry.taken_at;
            photo.camera.clone_from(&entry.camera);
            photo.fingerprint = Some(entry.fingerprint.clone());
        }
    }
    project
}

/// Número de lâminas sugerido para as fotos do projeto (~4 por lâmina; spec CB, regra 2), sem
/// ficar abaixo do mínimo de páginas dele (spec PR, regra 4).
pub fn suggestion(project: &Project) -> usize {
    suggestion_within(project.photos.len(), &project.spec, project.page_limits)
}

/// A sugestão para `photos` fotos, antes de importar (janela "Novo álbum").
pub fn suggestion_for(photos: usize, spec: &AlbumSpec) -> usize {
    suggest_spreads(photos, spec.edges(), &PlanOptions::default())
}

/// A sugestão sem ficar abaixo do mínimo de páginas (nem passar de uma lâmina por foto).
pub fn suggestion_within(photos: usize, spec: &AlbumSpec, limits: PageLimits) -> usize {
    let mut spreads = suggestion_for(photos, spec);
    let Some(min) = limits.min.and_then(|m| usize::try_from(m).ok()) else {
        return spreads;
    };
    while spec.edges().page_count(spreads) < min && spreads < photos.max(1) {
        spreads += 1;
    }
    spreads
}

/// Confere o número de lâminas pedido antes de montar, com as mensagens do plano (PL).
pub fn check_spreads(photos: usize, spreads: usize, spec: &AlbumSpec) -> Result<(), PlanError> {
    validate_plan(photos, spreads, spec.edges(), &PlanOptions::default())
}

/// Quantas fotos que o Lâmina lê há na pasta (e subpastas), sem decodificar nenhuma.
pub fn count_photos(folder: &Path) -> Result<usize, EditorError> {
    if !folder.is_dir() {
        return Err(EditorError::MissingFolder(folder.display().to_string()));
    }
    let photos = list_photos(folder).map_err(|source| MediaError::Io {
        path: folder.to_path_buf(),
        source,
    })?;
    Ok(photos
        .iter()
        .filter(|p| ImageFormat::from_path(p).is_some_and(ImageFormat::is_decodable))
        .count())
}

/// "Sugestão: 10 lâminas (cerca de 4 fotos por lâmina)."
pub fn suggestion_text(spreads: usize) -> String {
    let word = if spreads == 1 { "lâmina" } else { "lâminas" };
    format!("Sugestão: {spreads} {word} (cerca de {PHOTOS_PER_SPREAD} fotos por lâmina).")
}

/// O álbum inteiro: importa, sugere (se faltar o número de lâminas) e monta.
pub fn create_album(
    new: &NewAlbum,
    cache: &Cache,
    progress: &(dyn Fn(Step) + Sync),
) -> Result<(AutoBuild, Vec<(PathBuf, String)>), EditorError> {
    let name = new.name.as_deref();
    let mut imported = import_project(&new.folder, name, new.spec.clone(), cache, true, progress)?;
    imported.project.page_limits = new.limits;
    imported.project.export = new.export;
    progress(Step::Building);
    let spreads = new.spreads.unwrap_or_else(|| suggestion(&imported.project));
    let options = BuildOptions {
        reuse: new.reuse,
        preferences: new.preferences.clone(),
        ..BuildOptions::default()
    };
    let built = auto_build(&imported.project, spreads, &options)?;
    Ok((built, imported.failures))
}

/// Nome da pasta (caminho absoluto, sem tocar o disco); "album" se não houver.
pub fn folder_name(folder: &Path) -> String {
    let full = std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf());
    full.file_name()
        .map_or_else(|| "album".to_owned(), |n| n.to_string_lossy().into_owned())
}
