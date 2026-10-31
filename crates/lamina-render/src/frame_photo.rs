//! Os pixels da foto de um quadro (spec RD, regras 2 e 5; spec AF, regras 5 a 7): o pedaço da
//! foto reduzido com Lanczos3 e, com ajustes, girado em 1:1 e com o tom. A mesma função desenha a
//! exportação e a tela; sem ajustes, é o caminho de sempre.

use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::RgbaImage;
use lamina_core::{Crop, PhotoAdjust, PixelRegion, Rect, turned_box};

use crate::place::{Bled, Scale, region};
use crate::tone::apply_tone;
use crate::turn::{Plane, rotate_into, turned_source};

/// O que ler da foto e em que tamanho reduzir: a região e o tamanho da redução; com giro, a
/// caixa girada (em mm) e o ângulo.
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub region: PixelRegion,
    pub size: (u32, u32),
    turned: Option<((f64, f64), f64)>,
}

/// Quantos pixels inteiros cabem em `mm` na escala `per_mm` (pelo menos 1).
fn px(mm: f64, per_mm: f64) -> u32 {
    // Preso entre 1 e o maior u32, então o cast não trunca.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (mm * per_mm).ceil().clamp(1.0, f64::from(u32::MAX)) as u32;
    n
}

/// O plano do quadro `frame` (mm), estendido como `bled`, numa foto de `photo` pixels, para sair
/// com `out` pixels na escala `scale`.
pub fn plan(
    crop: &Crop,
    adjust: &PhotoAdjust,
    (frame, bled): (Rect, &Bled),
    photo: (f64, f64),
    (out, scale): ((u32, u32), Scale),
) -> Plan {
    if adjust.angle_tenths == 0 {
        return Plan {
            region: region(crop, frame, bled, photo),
            size: out,
            turned: None,
        };
    }
    let degrees = adjust.degrees();
    let box_mm = turned_box(bled.rect.width.get(), bled.rect.height.get(), degrees);
    Plan {
        region: turned_source(crop, (frame, bled), photo, degrees),
        size: (px(box_mm.0, scale.x), px(box_mm.1, scale.y)),
        turned: Some((box_mm, degrees)),
    }
}

/// Os pixels RGBA do quadro: `out` pixels cobrindo o quadro estendido, com o giro e o tom da foto.
pub fn frame_pixels(
    image: &RgbaImage,
    crop: &Crop,
    adjust: &PhotoAdjust,
    (frame, bled): (Rect, &Bled),
    (out, scale): ((u32, u32), Scale),
) -> Result<Vec<u8>, String> {
    let photo = (f64::from(image.width()), f64::from(image.height()));
    let p = plan(crop, adjust, (frame, bled), photo, (out, scale));
    let reduced = resize(image, p.region, p.size)?;
    let mut rgba = match p.turned {
        None => reduced,
        Some((box_mm, degrees)) => {
            let plane = Plane {
                width: p.size.0 as usize,
                height: p.size.1 as usize,
                rgba: &reduced,
            };
            rotate_into(&plane, box_mm, out, (scale.x, scale.y), degrees)
        }
    };
    apply_tone(&mut rgba, adjust);
    Ok(rgba)
}

/// Para o pedaço `r` de uma foto `full` virar `size` pixels sem ampliar, a foto inteira precisa
/// ter `full × size / r` pixels.
pub fn needed_size(r: PixelRegion, full: (f64, f64), size: (u32, u32)) -> (u32, u32) {
    let need = |whole: f64, part: f64, px: u32| {
        let v = (whole * f64::from(px) / part.max(1e-9)).ceil();
        // Preso ao maior u32, então o cast não trunca.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = v.clamp(1.0, f64::from(u32::MAX)) as u32;
        n
    };
    (
        need(full.0, r.width, size.0),
        need(full.1, r.height, size.1),
    )
}

/// Reduz (ou amplia) o pedaço `r` da foto para `size` pixels, com Lanczos3.
fn resize(image: &RgbaImage, r: PixelRegion, size: (u32, u32)) -> Result<Vec<u8>, String> {
    let src = ImageRef::new(
        image.width(),
        image.height(),
        image.as_raw(),
        PixelType::U8x4,
    )
    .map_err(|e| e.to_string())?;
    let mut dst = Image::new(size.0, size.1, PixelType::U8x4);
    // Foto opaca (quase toda foto): sem a cópia inteira que o redutor faz para tratar o alfa, e o
    // resultado é o mesmo (spec LV, regra 13).
    let opaque = image
        .as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .all(|px| px[3] == 255);
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3))
        .crop(r.x, r.y, r.width, r.height)
        .use_alpha(!opaque);
    Resizer::new()
        .resize(&src, &mut dst, &options)
        .map_err(|e| e.to_string())?;
    Ok(dst.into_vec())
}
