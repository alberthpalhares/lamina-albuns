//! As cópias reduzidas de uma foto lida uma vez só (spec TR, regras 2 e 3). A mesma leitura do
//! arquivo dá a impressão digital, as medidas, o EXIF, o perfil de cor e os pixels. A foto é
//! decodificada em RGB, reduzida para a imagem de tela e só então convertida de cor e girada; a
//! miniatura sai da imagem de tela. A exportação e o modo recorte continuam com a foto inteira
//! (`decode_in`; regra 14 do AGENTS).

use std::io::Cursor;
use std::path::{Path, PathBuf};

use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};
use lamina_core::ColorSpace;

use crate::color::{convert, problem_of};
use crate::fingerprint::fingerprint_of;
use crate::metadata::{exif_of, metadata_from};
use crate::thumbnail::{
    DerivedDirs, SCREEN_SIZE, THUMB_SIZE, derived_path, save_screen, save_thumb,
};
use crate::{MediaError, PhotoMetadata};

/// Uma foto aberta a partir do arquivo lido inteiro, antes de decodificar os pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Opened {
    pub fingerprint: String,
    pub metadata: PhotoMetadata,
    /// O perfil de cor embutido, se houver.
    pub icc: Option<Vec<u8>>,
    /// O aviso do perfil que não dá para ler (spec CT, seção 6), no mesmo trabalho.
    pub warning: Option<String>,
}

/// Abre a foto a partir dos `bytes` do arquivo: o que a importação guarda e o decodificador
/// pronto para os pixels. `path` serve só às mensagens; nada mais é lido do disco.
pub fn open(path: &Path, bytes: Vec<u8>) -> Result<(Opened, impl ImageDecoder), MediaError> {
    let fingerprint = fingerprint_of(&bytes);
    let exif = exif_of(&bytes);
    // O decodificador fica com os bytes (o de JPEG copia e solta a leitura).
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(MediaError::io(path))?
        .into_decoder()
        .map_err(MediaError::image(path))?;
    let metadata = metadata_from(decoder.dimensions(), exif.as_ref());
    let icc = decoder.icc_profile().ok().flatten();
    let warning = icc.as_deref().and_then(|icc| problem_of(path, icc));
    let opened = Opened {
        fingerprint,
        metadata,
        icc,
        warning,
    };
    Ok((opened, decoder))
}

/// Grava o que faltar entre a miniatura e a imagem de tela; só decodifica se faltar alguma.
/// Devolve o caminho da miniatura.
pub fn write_derived(
    path: &Path,
    decoder: impl ImageDecoder,
    opened: &Opened,
    dirs: DerivedDirs<'_>,
) -> Result<PathBuf, MediaError> {
    let thumb = derived_path(dirs.thumbs, &opened.fingerprint);
    let screen = dirs
        .screens
        .map(|d| derived_path(d, &opened.fingerprint))
        .filter(|s| !s.exists());
    if thumb.exists() && screen.is_none() {
        return Ok(thumb);
    }
    let full = DynamicImage::from_decoder(decoder).map_err(MediaError::image(path))?;
    let orientation = opened.metadata.exif_orientation;
    // A miniatura sai sempre da imagem de tela, com ou sem a imagem de tela pedida: a mesma
    // miniatura na janela e na linha de comando.
    let screen_image = reduced(full, opened.icc.as_deref(), orientation, SCREEN_SIZE);
    if let Some(screen) = screen {
        save_screen(&screen_image, &screen)?;
    }
    if !thumb.exists() {
        save_thumb(
            &screen_image.thumbnail(THUMB_SIZE, THUMB_SIZE),
            dirs.thumbs,
            &thumb,
        )?;
    }
    Ok(thumb)
}

/// Lê o arquivo uma vez e grava as cópias que faltam. Sem `fingerprint`, ela sai do arquivo; a
/// orientação vem do EXIF. Devolve o caminho da miniatura.
pub fn derive_file(
    path: &Path,
    fingerprint: Option<&str>,
    dirs: DerivedDirs<'_>,
) -> Result<PathBuf, MediaError> {
    let bytes = std::fs::read(path).map_err(MediaError::io(path))?;
    let (mut opened, decoder) = open(path, bytes)?;
    if let Some(fingerprint) = fingerprint {
        fingerprint.clone_into(&mut opened.fingerprint);
    }
    write_derived(path, decoder, &opened, dirs)
}

/// A foto reduzida para caber em `max_side`, em sRGB e em pé. Reduzir vem antes de converter a
/// cor e de girar, que assim trabalham na cópia pequena (regra 3).
pub fn reduced(
    full: DynamicImage,
    icc: Option<&[u8]>,
    exif_orientation: u8,
    max_side: u32,
) -> DynamicImage {
    let mut small = shrink(full, max_side);
    if icc.is_some() {
        small = DynamicImage::ImageRgba8(convert(small.into_rgba8(), icc, ColorSpace::Srgb));
    }
    if let Some(orientation) = Orientation::from_exif(exif_orientation) {
        small.apply_orientation(orientation);
    }
    small
}

/// A imagem reduzida para caber em `max_side`; a grande sai da memória aqui mesmo. Com o
/// `fast_image_resize` (média da área, SIMD; ADR 0006), 4 vezes mais rápido que o `thumbnail` do
/// `image`, que levava tanto quanto decodificar (spike `spikes/tr-decodificar`).
fn shrink(image: DynamicImage, max_side: u32) -> DynamicImage {
    let (w, h) = (image.width(), image.height());
    if w.max(h) <= max_side {
        return image;
    }
    let size = fit(w, h, max_side);
    let fast = match &image {
        DynamicImage::ImageRgb8(i) => box_resize(i.as_raw(), (w, h), size, PixelType::U8x3)
            .and_then(|raw| image::RgbImage::from_raw(size.0, size.1, raw))
            .map(DynamicImage::ImageRgb8),
        DynamicImage::ImageRgba8(i) => box_resize(i.as_raw(), (w, h), size, PixelType::U8x4)
            .and_then(|raw| image::RgbaImage::from_raw(size.0, size.1, raw))
            .map(DynamicImage::ImageRgba8),
        DynamicImage::ImageLuma8(i) => box_resize(i.as_raw(), (w, h), size, PixelType::U8)
            .and_then(|raw| image::GrayImage::from_raw(size.0, size.1, raw))
            .map(DynamicImage::ImageLuma8),
        // 16 bits e o resto: o caminho de antes.
        _ => None,
    };
    fast.unwrap_or_else(|| image.thumbnail(max_side, max_side))
}

/// O tamanho que cabe em `max_side` × `max_side` mantendo a proporção, como o `thumbnail` do
/// `image` calcula (as cópias de antes e as de agora têm as mesmas medidas).
// Medidas de imagem cabem com folga em u32, e a razão é positiva.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn fit(w: u32, h: u32, max_side: u32) -> (u32, u32) {
    let ratio = f64::from(max_side) / f64::from(w.max(h));
    let side = |v: u32| ((f64::from(v) * ratio).round() as u32).max(1);
    (side(w), side(h))
}

/// Reduz os pixels `raw` (`from`) para `to` pela média da área; `None` se o redutor recusar.
fn box_resize(raw: &[u8], from: (u32, u32), to: (u32, u32), kind: PixelType) -> Option<Vec<u8>> {
    let src = ImageRef::new(from.0, from.1, raw, kind).ok()?;
    let mut dst = Image::new(to.0, to.1, kind);
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Box))
        .use_alpha(false);
    Resizer::new().resize(&src, &mut dst, &options).ok()?;
    Some(dst.into_vec())
}
