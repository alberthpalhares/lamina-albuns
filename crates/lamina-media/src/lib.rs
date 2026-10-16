//! # lamina-media
//!
//! Importação de fotos:
//!
//! - [`format`]: formatos reconhecidos e listagem de pastas.
//! - [`metadata`]: dimensões, orientação, data de captura e câmera (EXIF), sem decodificar a foto.
//! - [`fingerprint`]: impressão digital rápida do conteúdo, para reencontrar fotos movidas.
//! - [`thumbnail`]: miniaturas e imagens de tela com a rotação EXIF aplicada.
//! - [`catalog`]: catálogo SQLite, que evita reprocessar fotos que não mudaram.
//! - [`import`]: tudo junto, em paralelo.

pub mod catalog;
pub mod decode;
pub mod fingerprint;
pub mod format;
pub mod import;
pub mod metadata;
pub mod photo_source;
pub mod thumbnail;

mod error;
#[cfg(test)]
mod testutil;

pub use catalog::{Catalog, CatalogEntry};
pub use decode::{decode_with_orientation, load_oriented};
pub use error::{MediaError, io_reason};
pub use format::{ImageFormat, list_photos};
pub use import::{ImportReport, Progress, default_cache_dir, import_folder, import_folder_with};
pub use metadata::{PhotoMetadata, read_metadata};
pub use photo_source::{Loaded, original_source, thumbnail_source};
pub use thumbnail::{
    DerivedDirs, SCREEN_SIZE, THUMB_SIZE, derived_path, ensure_derived, ensure_thumbnail,
};
