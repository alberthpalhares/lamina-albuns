//! Erros do editor, com as mensagens que a janela e a linha de comando mostram.

use lamina_core::CoreError;
use lamina_export::{ExportError, ProjectFileError};
use lamina_layout::{BuildError, LayoutError};
use lamina_media::MediaError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("A pasta {0} não existe.")]
    MissingFolder(String),
    #[error("Nenhuma foto encontrada em {0}.")]
    NoPhotos(String),
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error(transparent)]
    File(#[from] ProjectFileError),
    #[error(transparent)]
    Export(#[from] ExportError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error("Quadro ou lâmina que não existe.")]
    NoFrame,
    #[error("Nenhum projeto aberto.")]
    NoProject,
    #[error("O projeto ainda não tem arquivo: use Salvar como.")]
    NoFile,
    #[error("O álbum de exemplo não está em {0}.")]
    NoExample(String),
    #[error("Não foi possível copiar o álbum de exemplo para {path}: {reason}.")]
    CopyExample { path: String, reason: String },
    /// Acrescentar numa lâmina que já tem a capacidade dela (spec ML, regra 8).
    #[error("A lâmina {} já tem {capacity} fotos, o máximo que cabe nela.", spread + 1)]
    SpreadFull { spread: usize, capacity: usize },
    /// Acrescentar uma foto que já está na lâmina (spec ML, regra 9).
    #[error("Esta foto já está na lâmina {}.", spread + 1)]
    AlreadyInSpread { spread: usize },
}
