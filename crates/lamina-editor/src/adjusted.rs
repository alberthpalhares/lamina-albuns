//! A foto ajustada na tela (spec AF, regra 7): o quadro com giro ou tom é desenhado pela mesma
//! função da exportação (`lamina_render::frame_pixels`), a partir da imagem de tela (ou da
//! original, no recorte). Quadros sem ajuste continuam como hoje: a janela recorta a foto inteira.

use image::RgbaImage;
use lamina_core::{FrameContent, Project};
use lamina_render::{Scale, bleed, frame_pixels};

/// A foto de um quadro ajustado, pronta para a tela: cobre o quadro estendido pela sangria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Quantos pixels inteiros cabem em `mm` (pelo menos 1).
fn px(mm: f64, per_mm: f64) -> u32 {
    // Preso entre 1 e o maior u32, então o cast não trunca.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (mm * per_mm).round().clamp(1.0, f64::from(u32::MAX)) as u32;
    n
}

/// O quadro `frame` da lâmina `index` desenhado com os ajustes da foto, a `per_mm` pixels por mm,
/// a partir da imagem `screen` (largura, altura, RGBA). `None` sem foto, sem ajuste ou com a
/// imagem inválida.
pub fn adjusted_frame(
    project: &Project,
    (index, frame): (usize, usize),
    screen: (u32, u32, &[u8]),
    per_mm: f64,
) -> Option<AdjustedFrame> {
    let f = project.spreads.get(index)?.frames.get(frame)?;
    let FrameContent::Photo { photo, crop } = &f.content else {
        return None;
    };
    let adjust = project.photo(*photo).ok()?.adjust;
    if adjust.is_neutral() {
        return None;
    }
    let image = RgbaImage::from_raw(screen.0, screen.1, screen.2.to_vec())?;
    let rect = project.frame_rect(f);
    let bled = bleed(&project.spec, rect);
    let out = (
        px(bled.rect.width.get(), per_mm),
        px(bled.rect.height.get(), per_mm),
    );
    let scale = Scale {
        x: f64::from(out.0) / bled.rect.width.get(),
        y: f64::from(out.1) / bled.rect.height.get(),
    };
    let rgba = frame_pixels(&image, crop, &adjust, (rect, &bled), (out, scale)).ok()?;
    Some(AdjustedFrame {
        width: out.0,
        height: out.1,
        rgba,
    })
}
