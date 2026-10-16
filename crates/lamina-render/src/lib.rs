//! # lamina-render
//!
//! Renderizador único (ADR 0006): a mesma cena gera a prévia na tela e o arquivo de
//! impressão (spec `renderizacao`). Tela de desenho com `tiny-skia`; fotos reduzidas com
//! `fast_image_resize`. Texto e bordas chegam nos Marcos 2 e 3.

mod place;
mod render;
mod types;

pub use place::{Bled, PixelBox, Scale, bleed, pixels, region};
pub use render::render_spread;
pub use types::{MIN_PREVIEW, PhotoSource, RenderError, RenderTarget};

use lamina_core::AlbumSpec;

/// Tamanho em pixels de uma imagem a renderizar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderSize {
    pub width: u32,
    pub height: u32,
}

/// Tamanho de impressão de uma lâmina (com sangria), no DPI da especificação.
pub fn print_size(spec: &AlbumSpec) -> RenderSize {
    let (width, height) = spec.canvas_size_px();
    RenderSize { width, height }
}

/// Tamanho de uma prévia que cabe num quadrado de `max_side` pixels, mantendo a proporção.
// Os resultados ficam entre 0 e `max_side`, então cabem em u32.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn preview_size(spec: &AlbumSpec, max_side: u32) -> RenderSize {
    let canvas = spec.canvas_rect();
    let scale = f64::from(max_side) / canvas.width.get().max(canvas.height.get());
    RenderSize {
        width: ((canvas.width.get() * scale).round() as u32).max(1),
        height: ((canvas.height.get() * scale).round() as u32).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lamina_core::{Dpi, Mm};

    fn spec() -> AlbumSpec {
        AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap()
            .with_bleed(Mm(3.0))
            .unwrap()
    }

    #[test]
    fn tamanho_de_impressao_inclui_sangria() {
        assert_eq!(
            print_size(&spec()),
            RenderSize {
                width: 7157,
                height: 3614
            }
        );
    }

    #[test]
    fn previa_mantem_a_proporcao() {
        // 606 × 306 mm em no máximo 1200 px → 1200 × 606 px
        assert_eq!(
            preview_size(&spec(), 1200),
            RenderSize {
                width: 1200,
                height: 606
            }
        );
    }
}

#[cfg(test)]
#[path = "tests/rd.rs"]
mod rd;
