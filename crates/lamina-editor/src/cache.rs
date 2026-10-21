//! O cache do sistema: catálogo de fotos, miniaturas (320 px) e imagens de tela (1600 px).

use std::path::{Path, PathBuf};

use lamina_core::Photo;
use lamina_media::{Catalog, DerivedDirs, default_cache_dir, derived_path, ensure_derived_for};

use crate::EditorError;

/// Pastas do cache. A mesma em todo o app; os testes usam uma pasta temporária.
#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Cache { root: root.into() }
    }

    /// O cache padrão do sistema (o mesmo da linha de comando).
    pub fn system() -> Self {
        Cache::new(default_cache_dir())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn catalog_path(&self) -> PathBuf {
        self.root.join("catalogo.sqlite")
    }

    pub fn thumbs_dir(&self) -> PathBuf {
        self.root.join("miniaturas")
    }

    pub fn screens_dir(&self) -> PathBuf {
        self.root.join("telas")
    }

    pub fn open_catalog(&self) -> Result<Catalog, EditorError> {
        Ok(Catalog::open(&self.catalog_path())?)
    }

    /// A miniatura da foto, se já estiver no cache.
    pub fn thumbnail(&self, photo: &Photo) -> Option<PathBuf> {
        let fp = photo.fingerprint.as_deref()?;
        Some(derived_path(&self.thumbs_dir(), fp)).filter(|p| p.exists())
    }

    /// A imagem de tela da foto, se já estiver no cache.
    pub fn screen(&self, photo: &Photo) -> Option<PathBuf> {
        let fp = photo.fingerprint.as_deref()?;
        Some(derived_path(&self.screens_dir(), fp)).filter(|p| p.exists())
    }

    /// Gera a miniatura e a imagem de tela que faltarem (projeto aberto em outra máquina ou com
    /// o cache limpo), lendo a foto uma vez só (spec TR, regra 2). Foto sem impressão digital
    /// gravada ganha uma agora.
    pub fn ensure_images(&self, photo: &Photo) -> Result<(), EditorError> {
        let (thumbs, screens) = (self.thumbs_dir(), self.screens_dir());
        let dirs = DerivedDirs {
            thumbs: &thumbs,
            screens: Some(&screens),
        };
        ensure_derived_for(&photo.path, photo.fingerprint.as_deref(), dirs)?;
        Ok(())
    }
}
