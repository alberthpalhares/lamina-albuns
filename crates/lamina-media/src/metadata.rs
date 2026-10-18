//! Metadados de uma foto, lidos sem decodificar os pixels.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{Exif, In, Tag, Value};
use image::ImageReader;
use lamina_core::CaptureTime;

use crate::MediaError;

/// O que o Lamina precisa saber de uma foto antes de diagramar.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoMetadata {
    /// Largura e altura em pixels, **já com a rotação EXIF aplicada**.
    pub width: u32,
    pub height: u32,
    /// Orientação EXIF (1 a 8); 1 quando ausente.
    pub exif_orientation: u8,
    pub taken_at: Option<CaptureTime>,
    pub camera: Option<String>,
}

/// Lê dimensões do cabeçalho e metadados EXIF. Fotos sem EXIF são aceitas.
pub fn read_metadata(path: &Path) -> Result<PhotoMetadata, MediaError> {
    let (raw_w, raw_h) = ImageReader::open(path)
        .map_err(MediaError::io(path))?
        .with_guessed_format()
        .map_err(MediaError::io(path))?
        .into_dimensions()
        .map_err(MediaError::image(path))?;
    let exif = read_exif(path);
    let exif_orientation = exif
        .as_ref()
        .and_then(|e| e.get_field(Tag::Orientation, In::PRIMARY))
        .and_then(|f| f.value.get_uint(0))
        .and_then(|v| u8::try_from(v).ok())
        .filter(|v| (1..=8).contains(v))
        .unwrap_or(1);
    // Orientações 5 a 8 giram a foto 90°: largura e altura trocam.
    let (width, height) = if exif_orientation >= 5 {
        (raw_h, raw_w)
    } else {
        (raw_w, raw_h)
    };
    let text = |tag| exif.as_ref().and_then(|e| ascii(e, tag));
    let taken_at = text(Tag::DateTimeOriginal)
        .or_else(|| text(Tag::DateTime))
        .and_then(|t| CaptureTime::from_exif(&t));
    Ok(PhotoMetadata {
        width,
        height,
        exif_orientation,
        taken_at,
        camera: text(Tag::Model),
    })
}

fn read_exif(path: &Path) -> Option<Exif> {
    let file = File::open(path).ok()?;
    exif::Reader::new()
        .read_from_container(&mut BufReader::new(file))
        .ok()
}

/// Primeiro texto ASCII de um campo, sem espaços e nulos nas pontas.
fn ascii(exif: &Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    let Value::Ascii(values) = &field.value else {
        return None;
    };
    let text = String::from_utf8_lossy(values.first()?);
    let text = text.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{Exif as TestExif, write_jpeg};

    #[test]
    fn le_data_camera_e_dimensoes() {
        let dir = tempfile::tempdir().unwrap();
        let exif = TestExif::new(1, "2026:09:12 15:00:40", "Camera de Teste");
        let path = write_jpeg(dir.path(), "a.jpg", 60, 40, Some(&exif));
        let m = read_metadata(&path).unwrap();
        assert_eq!((m.width, m.height), (60, 40));
        assert_eq!(m.taken_at.unwrap().to_string(), "2026-09-12 15:00:40");
        assert_eq!(m.camera.as_deref(), Some("Camera de Teste"));
    }

    #[test]
    fn orientacao_6_troca_largura_e_altura() {
        let dir = tempfile::tempdir().unwrap();
        let exif = TestExif::new(6, "2026:09:12 15:00:40", "Camera de Teste");
        let path = write_jpeg(dir.path(), "girada.jpg", 60, 40, Some(&exif));
        let m = read_metadata(&path).unwrap();
        assert_eq!(m.exif_orientation, 6);
        assert_eq!((m.width, m.height), (40, 60));
    }

    #[test]
    fn foto_sem_exif_e_aceita() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "simples.jpg", 30, 30, None);
        let m = read_metadata(&path).unwrap();
        assert_eq!(m.exif_orientation, 1);
        assert_eq!((m.taken_at, m.camera), (None, None));
    }

    #[test]
    fn arquivo_que_nao_e_imagem_da_erro() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("falso.jpg");
        std::fs::write(&path, b"isto nao e uma imagem").unwrap();
        assert!(matches!(
            read_metadata(&path),
            Err(MediaError::Image { .. })
        ));
    }
}
