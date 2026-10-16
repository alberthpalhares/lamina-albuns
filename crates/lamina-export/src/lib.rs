//! # lamina-export
//!
//! Exportação para impressão (spec `exportacao`): JPEG por lâmina ou por página, com sangria,
//! perfil sRGB e DPI, nomes ordenáveis e pré-impressão. TIFF, PDF/X-4 e Adobe RGB chegam no
//! Marco 3.

mod export;
mod jobs;
mod jpeg;
mod preflight;
mod previews;
mod project_file;
mod range;
mod write;

pub use export::{DEFAULT_QUALITY, ExportOptions, ExportReport, export, output_names};
pub use jpeg::srgb_profile;
pub use preflight::{Warning, WarningKind, preflight};
pub use previews::write_previews;
pub use project_file::{
    PROJECT_EXT, ProjectFileError, open_project, project_file_name, read_project, save_project,
};
pub use range::parse_range;

use lamina_render::RenderError;
use thiserror::Error;

/// Formatos de saída.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Jpeg,
    /// Prévias de tela (sem perda).
    Png,
    Tiff,
    Pdf,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Png => "png",
            ExportFormat::Tiff => "tif",
            ExportFormat::Pdf => "pdf",
        }
    }
}

/// Erros da exportação, com as mensagens da spec.
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("A qualidade do JPEG vai de 1 a 100, não {0}.")]
    Quality(u8),
    #[error("Intervalo inválido: \"{0}\". Use, por exemplo, 1, 2, 5-7.")]
    BadRange(String),
    #[error("O intervalo pede a lâmina {asked}; o projeto tem {count}.")]
    RangeTooBig { asked: usize, count: usize },
    #[error("Já existe {name} em {folder}; use sobrescrever para substituir.")]
    Exists { name: String, folder: String },
    #[error("O projeto não tem lâminas para exportar.")]
    NoSpreads,
    #[error("Não foi possível criar a pasta {folder}: {reason}.")]
    Folder { folder: String, reason: String },
    #[error("Não foi possível gravar {path}: {reason}.")]
    Write { path: String, reason: String },
    #[error("Não foi possível codificar o JPEG: {0}.")]
    Encode(String),
    #[error(transparent)]
    Render(#[from] RenderError),
    /// A exportação parou no meio: `done` são os arquivos que ficaram gravados.
    #[error("{source}")]
    Stopped {
        done: Vec<std::path::PathBuf>,
        source: Box<ExportError>,
    },
}

/// Nome do arquivo da lâmina `index` (começando em 1), com zeros à esquerda para que a
/// ordem alfabética seja a ordem do álbum.
pub fn spread_file_name(index: usize, total: usize, format: ExportFormat) -> String {
    numbered("lamina", index, total, format)
}

/// Nome do arquivo da página `index` (começando em 1) num álbum de `total` páginas.
pub fn page_file_name(index: usize, total: usize, format: ExportFormat) -> String {
    numbered("pagina", index, total, format)
}

fn numbered(prefix: &str, index: usize, total: usize, format: ExportFormat) -> String {
    let digits = total.max(1).to_string().len().max(2);
    format!("{prefix}-{index:0digits$}.{}", format.extension())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomes_ordenaveis() {
        assert_eq!(spread_file_name(1, 20, ExportFormat::Jpeg), "lamina-01.jpg");
        assert_eq!(
            spread_file_name(7, 120, ExportFormat::Tiff),
            "lamina-007.tif"
        );
        assert_eq!(spread_file_name(3, 5, ExportFormat::Pdf), "lamina-03.pdf");
        assert_eq!(page_file_name(4, 38, ExportFormat::Jpeg), "pagina-04.jpg");
    }
}

#[cfg(test)]
#[path = "tests/ex.rs"]
mod ex;
