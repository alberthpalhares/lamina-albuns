//! Tipos públicos da renderização (spec `renderizacao`): alvo, fonte de fotos e erros.

use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{CoreError, Photo};
use thiserror::Error;

/// Menor lado aceito para uma prévia.
pub const MIN_PREVIEW: u32 = 16;

/// Tamanho da imagem pedida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    /// No DPI da especificação, com a sangria.
    Print,
    /// Cabendo num quadrado de `max_side` pixels.
    Preview { max_side: u32 },
}

/// Quem entrega cada foto já lida e na orientação certa. `needed` é o tamanho que a **foto
/// inteira** precisa ter para o quadro sair nítido (o recorte e o zoom já contados): a fonte
/// pode usar uma versão menor (miniatura) quando ela já basta.
pub trait PhotoSource: Sync {
    fn load(&self, photo: &Photo, needed: (u32, u32)) -> Result<Arc<RgbaImage>, String>;
}

impl<F> PhotoSource for F
where
    F: Fn(&Photo, (u32, u32)) -> Result<Arc<RgbaImage>, String> + Sync,
{
    fn load(&self, photo: &Photo, needed: (u32, u32)) -> Result<Arc<RgbaImage>, String> {
        self(photo, needed)
    }
}

/// Erros da renderização, com as mensagens da spec.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("A lâmina {number} não existe; o projeto tem {count}.")]
    NoSpread { number: usize, count: usize },
    #[error("A prévia precisa ter pelo menos {} pixels.", MIN_PREVIEW)]
    PreviewTooSmall,
    #[error("Não foi possível ler a foto {name}: {reason}.")]
    Photo { name: String, reason: String },
    #[error("Não foi possível reduzir a foto {name} para o quadro: {reason}.")]
    Resize { name: String, reason: String },
    #[error("A imagem de {width} × {height} pixels não pôde ser criada.")]
    Canvas { width: u32, height: u32 },
    #[error(transparent)]
    Core(#[from] CoreError),
}
