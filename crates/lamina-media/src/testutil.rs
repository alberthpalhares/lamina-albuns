//! Ajudantes de teste: gera JPEGs pequenos, com ou sem EXIF, sem depender de arquivos externos.

use std::path::{Path, PathBuf};

use image::{ImageFormat, Rgb, RgbImage};

/// Campos EXIF que os testes precisam.
pub struct Exif {
    orientation: u16,
    datetime: String,
    model: String,
    /// Exposição (spec AF, regra 12): velocidade, abertura e distância focal em frações, ISO e
    /// espaço de cor (1 = sRGB).
    exposure: Option<Exposure>,
}

/// Os campos da câmera de um EXIF de teste.
pub struct Exposure {
    pub shutter: (u32, u32),
    pub aperture: (u32, u32),
    pub focal: (u32, u32),
    pub iso: u16,
    pub color: u16,
}

impl Exif {
    pub fn new(orientation: u16, datetime: &str, model: &str) -> Self {
        Exif {
            orientation,
            datetime: datetime.to_owned(),
            model: model.to_owned(),
            exposure: None,
        }
    }

    /// O mesmo EXIF com a exposição da câmera.
    pub fn with_exposure(self, exposure: Exposure) -> Self {
        Exif {
            exposure: Some(exposure),
            ..self
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
        let model_at = 8 + 2 + 3 * 12 + 4;
        let exif_ifd_at = model_at + model.len();
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
        // Exif IFD: as entradas em ordem de etiqueta; os valores grandes logo depois.
        let n: u16 = if self.exposure.is_some() { 6 } else { 1 };
        let data_at = exif_ifd_at + 2 + 12 * usize::from(n) + 4;
        let mut data: Vec<u8> = Vec::new();
        let rational = |data: &mut Vec<u8>, (num, den): (u32, u32)| {
            let at = data_at + data.len();
            data.extend(num.to_le_bytes());
            data.extend(den.to_le_bytes());
            u32_of(at)
        };
        t.extend(n.to_le_bytes());
        if let Some(e) = &self.exposure {
            let shutter = rational(&mut data, e.shutter);
            let aperture = rational(&mut data, e.aperture);
            entry(&mut t, 0x829A, 5, 1, shutter);
            entry(&mut t, 0x829D, 5, 1, aperture);
            entry(&mut t, 0x8827, 3, 1, u32::from(e.iso));
            let date = data_at + data.len();
            data.extend(&date_bytes(&self.datetime));
            entry(
                &mut t,
                0x9003,
                2,
                u32_of(self.datetime.len() + 1),
                u32_of(date),
            );
            let focal = rational(&mut data, e.focal);
            entry(&mut t, 0x920A, 5, 1, focal);
            entry(&mut t, 0xA001, 3, 1, u32::from(e.color));
        } else {
            let date = data_at;
            data.extend(&date_bytes(&self.datetime));
            entry(
                &mut t,
                0x9003,
                2,
                u32_of(self.datetime.len() + 1),
                u32_of(date),
            );
        }
        t.extend(0u32.to_le_bytes());
        t.extend(data);
        t
    }
}

/// O texto da data com o nulo do fim, em tamanho par (os valores do TIFF começam em par).
fn date_bytes(datetime: &str) -> Vec<u8> {
    let mut date = datetime.as_bytes().to_vec();
    date.push(0);
    if date.len() % 2 == 1 {
        date.push(0);
    }
    date
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
