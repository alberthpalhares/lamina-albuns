//! Ajudantes de teste: gera JPEGs pequenos, com ou sem EXIF, sem depender de arquivos externos.

use std::path::{Path, PathBuf};

use image::{ImageFormat, Rgb, RgbImage};

/// Campos EXIF que os testes precisam.
pub struct Exif {
    orientation: u16,
    datetime: String,
    model: String,
}

impl Exif {
    pub fn new(orientation: u16, datetime: &str, model: &str) -> Self {
        Exif {
            orientation,
            datetime: datetime.to_owned(),
            model: model.to_owned(),
        }
    }

    /// Bloco TIFF (little-endian) com IFD0 = Model, Orientation, ponteiro Exif; e Exif IFD =
    /// DateTimeOriginal.
    fn tiff(&self) -> Vec<u8> {
        let mut model = self.model.as_bytes().to_vec();
        model.push(0);
        if model.len() % 2 == 1 {
            model.push(0);
        }
        let mut date = self.datetime.as_bytes().to_vec();
        date.push(0);
        let model_at = 8 + 2 + 3 * 12 + 4;
        let exif_ifd_at = model_at + model.len();
        let date_at = exif_ifd_at + 2 + 12 + 4;
        let u32_of = |v: usize| u32::try_from(v).unwrap();
        let mut t = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
        let entry = |t: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: u32| {
            t.extend(tag.to_le_bytes());
            t.extend(kind.to_le_bytes());
            t.extend(count.to_le_bytes());
            t.extend(value.to_le_bytes());
        };
        t.extend(3u16.to_le_bytes());
        entry(
            &mut t,
            0x0110,
            2,
            u32_of(self.model.len() + 1),
            u32_of(model_at),
        );
        entry(&mut t, 0x0112, 3, 1, u32::from(self.orientation));
        entry(&mut t, 0x8769, 4, 1, u32_of(exif_ifd_at));
        t.extend(0u32.to_le_bytes());
        t.extend(&model);
        t.extend(1u16.to_le_bytes());
        entry(&mut t, 0x9003, 2, u32_of(date.len()), u32_of(date_at));
        t.extend(0u32.to_le_bytes());
        t.extend(&date);
        t
    }
}

/// Grava um JPEG `width` × `height` em `dir/name`, opcionalmente com EXIF.
pub fn write_jpeg(dir: &Path, name: &str, width: u32, height: u32, exif: Option<&Exif>) -> PathBuf {
    let img = RgbImage::from_fn(width, height, |x, y| {
        Rgb([
            u8::try_from(x % 256).unwrap(),
            u8::try_from(y % 256).unwrap(),
            128,
        ])
    });
    let mut jpeg = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut jpeg), ImageFormat::Jpeg)
        .unwrap();
    if let Some(exif) = exif {
        let tiff = exif.tiff();
        let len = u16::try_from(2 + 6 + tiff.len()).unwrap();
        let mut app1 = vec![0xFF, 0xE1];
        app1.extend(len.to_be_bytes());
        app1.extend(b"Exif\0\0");
        app1.extend(tiff);
        jpeg.splice(2..2, app1);
    }
    let path = dir.join(name);
    std::fs::write(&path, jpeg).unwrap();
    path
}
