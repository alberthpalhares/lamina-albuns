//! As imagens dos textos na tela (spec TX): o `lamina-render` desenha cada quadro de texto com o
//! mesmo código da impressão; aqui só se guarda o resultado para não desenhar de novo a cada
//! atualização da janela.

use std::cell::RefCell;
use std::collections::HashMap;

use lamina_core::TextBox;
use lamina_editor::text_image;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// Largura, em pixels, da lâmina grande para desenhar os textos: nítido mesmo em telas de alta
/// densidade. Miniaturas usam [`SMALL`].
pub const BIG: f64 = 2400.0;
pub const SMALL: f64 = 320.0;
/// Quantas imagens ficam guardadas antes de recomeçar.
const KEEP: usize = 400;

thread_local! {
    static CACHE: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
}

/// O texto desenhado num quadro de `size_mm`, com `px` pixels.
pub fn picture(text: &TextBox, size_mm: (f64, f64), px: (u32, u32)) -> Option<Image> {
    let key = format!("{text:?}|{size_mm:?}|{px:?}");
    if let Some(img) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return Some(img);
    }
    let rgba = text_image(text, size_mm, px)?;
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
    );
    let img = Image::from_rgba8_premultiplied(buffer);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= KEEP {
            c.clear();
        }
        c.insert(key, img.clone());
    });
    Some(img)
}

/// Pixels de uma medida em frações da lâmina, numa lâmina de `width` pixels (pelo menos 1).
// Frações entre 0 e 1 vezes alguns milhares de pixels: cabe em u32.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn pixels(fraction: f64, width: f64) -> u32 {
    ((fraction * width).round() as u32).max(1)
}
