//! Leitura dos pixels de uma foto na orientação certa (rotação EXIF aplicada), para a
//! renderização (spec `renderizacao`, RD-B10) e para as miniaturas.

use std::path::Path;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};
use lamina_core::ColorSpace;

use crate::color::convert;
use crate::{MediaError, read_metadata};

/// Decodifica a foto em sRGB (a tela; spec CT, regra 1) e aplica a orientação EXIF
/// `exif_orientation` (1 a 8).
pub fn decode_with_orientation(
    path: &Path,
    exif_orientation: u8,
) -> Result<DynamicImage, MediaError> {
    decode_in(path, exif_orientation, ColorSpace::Srgb)
}

/// Decodifica a foto convertendo do perfil dela para `space` (spec CT, regra 2) e aplica a
/// orientação EXIF. Foto sem perfil em sRGB passa intacta (regra 7).
pub fn decode_in(
    path: &Path,
    exif_orientation: u8,
    space: ColorSpace,
) -> Result<DynamicImage, MediaError> {
    let mut decoder = ImageReader::open(path)
        .map_err(MediaError::io(path))?
        .with_guessed_format()
        .map_err(MediaError::io(path))?
        .into_decoder()
        .map_err(MediaError::image(path))?;
    let icc = decoder.icc_profile().ok().flatten();
    let mut image = DynamicImage::from_decoder(decoder).map_err(MediaError::image(path))?;
    if icc.is_some() || space != ColorSpace::Srgb {
        image = DynamicImage::ImageRgba8(convert(to_rgba(image), icc.as_deref(), space));
    }
    if let Some(orientation) = Orientation::from_exif(exif_orientation) {
        image.apply_orientation(orientation);
    }
    Ok(image)
}

/// A imagem em RGBA. Fotos e cópias do cache são JPEG em RGB; a conversão direta leva uma fração
/// do tempo da genérica do `image` (medido: 23 ms por imagem de tela; spec LV, regra 3).
pub fn to_rgba(image: DynamicImage) -> RgbaImage {
    let DynamicImage::ImageRgb8(rgb) = image else {
        return image.into_rgba8();
    };
    let (width, height) = rgb.dimensions();
    let mut out = vec![255_u8; rgb.as_raw().len() / 3 * 4];
    let (dst, src) = (out.as_chunks_mut::<4>().0, rgb.as_raw().as_chunks::<3>().0);
    for (d, s) in dst.iter_mut().zip(src) {
        d[..3].copy_from_slice(s);
    }
    RgbaImage::from_raw(width, height, out).unwrap_or_default()
}

/// Lê a orientação do próprio arquivo e devolve os pixels em pé, em RGBA, em sRGB.
pub fn load_oriented(path: &Path) -> Result<RgbaImage, MediaError> {
    load_oriented_in(path, ColorSpace::Srgb)
}

/// Como [`load_oriented`], no espaço de cor da saída (spec CT, regra 2).
pub fn load_oriented_in(path: &Path, space: ColorSpace) -> Result<RgbaImage, MediaError> {
    let orientation = read_metadata(path)?.exif_orientation;
    Ok(to_rgba(decode_in(path, orientation, space)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{Exif, write_jpeg};

    #[test]
    fn rd_b10_foto_girada_pelo_exif_fica_em_pe() {
        // A foto de teste guarda x no vermelho e y no verde. Gravada deitada (60 × 40) com
        // orientação 6: em pé, o canto de cima à esquerda é o de baixo à esquerda do arquivo.
        let dir = tempfile::tempdir().unwrap();
        let exif = Exif::new(6, "2026:09:12 15:00:40", "Camera de Teste");
        let path = write_jpeg(dir.path(), "deitada.jpg", 60, 40, Some(&exif));
        let image = load_oriented(&path).unwrap();
        assert_eq!(
            image.dimensions(),
            (40, 60),
            "em pé: altura maior que a largura"
        );
        let top_left = image.get_pixel(0, 0);
        let top_right = image.get_pixel(39, 0);
        assert!(
            top_left[1] > 30,
            "em cima à esquerda vem de y alto: {top_left:?}"
        );
        assert!(
            top_right[1] < 10 && top_right[0] < 10,
            "em cima à direita vem da origem: {top_right:?}"
        );
    }

    #[test]
    fn foto_sem_exif_fica_como_esta() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "normal.jpg", 60, 40, None);
        assert_eq!(load_oriented(&path).unwrap().dimensions(), (60, 40));
    }

    #[test]
    fn arquivo_que_nao_e_foto_e_erro_de_imagem() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("falsa.jpg");
        std::fs::write(&path, b"isto nao e uma foto").unwrap();
        assert!(matches!(
            load_oriented(&path),
            Err(MediaError::Image { .. })
        ));
    }
}
