//! O documento do projeto: fotos vinculadas, lâminas e quadros.
//!
//! O projeto é salvo como JSON versionado (ADR 0004). As fotos **não** são embutidas:
//! o documento guarda o caminho relativo e uma impressão digital para reencontrar o
//! arquivo se ele mudar de pasta.

mod content;
mod ids;
mod migration;
mod paths;
mod photo;
mod print;
mod project;
mod text;

pub use content::{Border, Crop, Frame, FrameContent, PixelRegion, Rgb, Spread};
pub use ids::{FrameId, PhotoId, SpreadId};
pub use paths::{lexical_path, relative_path, validate_project_name};
pub use photo::{Orientation, Photo};
pub use print::{ExportPrefs, OutputFormat, PageLimits};
pub use project::{CURRENT_SCHEMA_VERSION, DEFAULT_GAP, Project};
pub use text::{
    DEFAULT_FONT, LINE_SPACING, MAX_TEXT_CHARS, TEXT_SIZE_PT, TextAlign, TextBox, TextVAlign,
    clean_text,
};
