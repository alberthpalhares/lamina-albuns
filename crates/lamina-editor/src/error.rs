//! Erros do editor, com as mensagens que a janela e a linha de comando mostram.

use lamina_core::CoreError;
use lamina_export::ProjectFileError;
use lamina_layout::BuildError;
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
    Core(#[from] CoreError),
    #[error("Nenhum projeto aberto.")]
    NoProject,
    #[error("O projeto ainda não tem arquivo: use Salvar como.")]
    NoFile,
}
