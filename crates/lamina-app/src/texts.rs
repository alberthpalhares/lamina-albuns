//! As imagens dos textos na tela (spec TX): o `lamina-render` desenha cada quadro de texto com o
//! mesmo código da impressão; aqui só se guarda o resultado para não desenhar de novo a cada
//! atualização da janela.

use std::cell::RefCell;

use lamina_core::TextBox;
use lamina_core::byte_cache::ByteCache;
use lamina_editor::text_image;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// Largura, em pixels, da lâmina grande para desenhar os textos: nítido mesmo em telas de alta
/// densidade. Miniaturas usam [`SMALL`].
pub const BIG: f64 = 2400.0;
pub const SMALL: f64 = 320.0;
/// Quantos bytes de imagens ficam guardados: cada tecla desenha um texto novo, e uma lâmina
/// inteira de texto a 2400 px tem uns 10 MB (spec LV, regra 4).
const KEEP_BYTES: usize = 96 * 1024 * 1024;

thread_local! {
    static CACHE: RefCell<ByteCache<String, Image>> = RefCell::new(ByteCache::new(KEEP_BYTES));
}

/// Outro álbum: as imagens do anterior vão embora.
pub fn clear() {
    CACHE.with(|c| c.borrow_mut().clear());
}

/// O texto desenhado num quadro de `size_mm`, com `px` pixels.
pub fn picture(text: &TextBox, size_mm: (f64, f64), px: (u32, u32)) -> Option<Image> {
    let key = format!("{text:?}|{size_mm:?}|{px:?}");
    if let Some(img) = CACHE.with(|c| c.borrow_mut().get(&key)) {
        return Some(img);
    }
    let rgba = text_image(text, size_mm, px)?;
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
    );
    let img = Image::from_rgba8_premultiplied(buffer);
    let bytes = rgba.as_raw().len();
    CACHE.with(|c| c.borrow_mut().insert(key, img.clone(), bytes));
    Some(img)
}

/// Pixels de uma medida em frações da lâmina, numa lâmina de `width` pixels (pelo menos 1).
// Frações entre 0 e 1 vezes alguns milhares de pixels: cabe em u32.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn pixels(fraction: f64, width: f64) -> u32 {
    ((fraction * width).round() as u32).max(1)
}
