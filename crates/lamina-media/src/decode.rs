//! Leitura dos pixels de uma foto na orientação certa (rotação EXIF aplicada), para a
//! renderização (spec `renderizacao`, RD-B10) e para as miniaturas.

use std::path::Path;

use image::metadata::Orientation;
use image::{DynamicImage, ImageReader, RgbaImage};

use crate::{MediaError, read_metadata};

/// Decodifica a foto e aplica a orientação EXIF `exif_orientation` (1 a 8).
pub fn decode_with_orientation(
    path: &Path,
    exif_orientation: u8,
) -> Result<DynamicImage, MediaError> {
    let mut image = ImageReader::open(path)
        .map_err(MediaError::io(path))?
        .with_guessed_format()
        .map_err(MediaError::io(path))?
        .decode()
        .map_err(MediaError::image(path))?;
    if let Some(orientation) = Orientation::from_exif(exif_orientation) {
        image.apply_orientation(orientation);
    }
    Ok(image)
}

/// Lê a orientação do próprio arquivo e devolve os pixels em pé, em RGBA.
pub fn load_oriented(path: &Path) -> Result<RgbaImage, MediaError> {
    let orientation = read_metadata(path)?.exif_orientation;
    Ok(decode_with_orientation(path, orientation)?.into_rgba8())
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
