//! As informações da câmera de uma foto (spec AF, regra 12): lidas do arquivo na hora em que a
//! foto é selecionada, sem decodificar a imagem e sem guardar no projeto. Sem a informação, a
//! linha não aparece.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{Exif, In, Tag, Value};

/// O que o painel mostra da câmera, já como texto ("1/200 s", "f/2,8", "ISO 100", "50 mm").
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CameraDetails {
    pub shutter: Option<String>,
    pub aperture: Option<String>,
    pub iso: Option<String>,
    pub focal: Option<String>,
    /// "sRGB" ou "Adobe RGB", quando o arquivo diz.
    pub color: Option<String>,
}

/// Lê as informações da câmera do arquivo. Sem EXIF (ou sem o arquivo), tudo vazio.
pub fn camera_details(path: &Path) -> CameraDetails {
    let Some(exif) = File::open(path).ok().and_then(|f| {
        exif::Reader::new()
            .read_from_container(&mut BufReader::new(f))
            .ok()
    }) else {
        return CameraDetails::default();
    };
    CameraDetails {
        shutter: rational(&exif, Tag::ExposureTime).map(shutter),
        aperture: rational(&exif, Tag::FNumber).map(|v| format!("f/{}", decimal(v))),
        iso: uint(&exif, Tag::PhotographicSensitivity).map(|v| format!("ISO {v}")),
        focal: rational(&exif, Tag::FocalLength).map(|v| format!("{} mm", decimal(v))),
        color: color(&exif),
    }
}

/// Número com até uma casa e vírgula: 2.8 → "2,8"; 50 → "50".
fn decimal(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    format!("{r}").replace('.', ",")
}

/// 0,005 → "1/200 s"; 2 → "2 s"; 0,4 → "0,4 s".
fn shutter(v: f64) -> String {
    if v > 0.0 && v < 0.3 {
        format!("1/{} s", (1.0 / v).round())
    } else {
        format!("{} s", decimal(v))
    }
}

fn rational(exif: &Exif, tag: Tag) -> Option<f64> {
    match &exif.get_field(tag, In::PRIMARY)?.value {
        Value::Rational(v) => v
            .first()
            .filter(|r| r.denom != 0)
            .map(|r| f64::from(r.num) / f64::from(r.denom)),
        _ => None,
    }
}

fn uint(exif: &Exif, tag: Tag) -> Option<u32> {
    exif.get_field(tag, In::PRIMARY)?.value.get_uint(0)
}

/// O espaço de cor do EXIF: 1 é sRGB; "não calibrado" com o índice R03 é Adobe RGB.
fn color(exif: &Exif) -> Option<String> {
    match uint(exif, Tag::ColorSpace)? {
        1 => Some("sRGB".to_owned()),
        0xFFFF => {
            let index = exif.get_field(Tag::InteroperabilityIndex, In::PRIMARY)?;
            let r03 = matches!(&index.value, Value::Ascii(v) if v.first().is_some_and(|s| s.starts_with(b"R03")));
            r03.then(|| "Adobe RGB".to_owned())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{Exif as TestExif, Exposure, write_jpeg};

    #[test]
    fn af_b21_informacoes_da_camera() {
        let dir = tempfile::tempdir().unwrap();
        let exif =
            TestExif::new(1, "2026:09:12 15:00:40", "Camera de Teste").with_exposure(Exposure {
                shutter: (1, 200),
                aperture: (28, 10),
                focal: (50, 1),
                iso: 100,
                color: 1,
            });
        let with = write_jpeg(dir.path(), "a.jpg", 60, 40, Some(&exif));
        let d = camera_details(&with);
        assert_eq!(d.shutter.as_deref(), Some("1/200 s"));
        assert_eq!(d.iso.as_deref(), Some("ISO 100"));
        assert_eq!(d.aperture.as_deref(), Some("f/2,8"));
        assert_eq!(d.focal.as_deref(), Some("50 mm"));
        assert_eq!(d.color.as_deref(), Some("sRGB"));
        // A data e a câmera continuam lendo do mesmo EXIF.
        let m = crate::read_metadata(&with).unwrap();
        assert_eq!(m.camera.as_deref(), Some("Camera de Teste"));
        assert!(m.taken_at.is_some());
        // Sem EXIF (ou sem o arquivo): nenhuma linha.
        let without = write_jpeg(dir.path(), "b.jpg", 60, 40, None);
        assert_eq!(camera_details(&without), CameraDetails::default());
        assert_eq!(
            camera_details(&dir.path().join("sumiu.jpg")),
            CameraDetails::default()
        );
    }

    #[test]
    fn velocidades_longas_e_curtas() {
        assert_eq!(shutter(0.004), "1/250 s");
        assert_eq!(shutter(2.0), "2 s");
        assert_eq!(shutter(0.5), "0,5 s");
    }
}
