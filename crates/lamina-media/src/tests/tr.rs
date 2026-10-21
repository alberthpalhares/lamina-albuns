//! Cenários da spec `travamento-com-muitas-fotos` (TR) na leitura das fotos: B3 e B5.

use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbImage, RgbaImage};
use lamina_core::ColorSpace;
use moxcms::ColorProfile;

use crate::fingerprint::fingerprint;
use crate::import::process_bytes;
use crate::testutil::{Exif, write_jpeg};
use crate::thumbnail::{DerivedDirs, SCREEN_SIZE, THUMB_SIZE, derived_path};
use crate::{decode_with_orientation, read_metadata};

/// Degradê suave em sRGB, `w × h`.
fn gradient(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        let v = |n: u32| u8::try_from(40 + n % 170).unwrap();
        image::Rgb([v(x * 170 / w), v(y * 170 / h), v((x + y) * 85 / (w + h))])
    })
}

/// Um JPEG com o perfil `icc` embutido e o EXIF com a orientação `orientation`.
fn jpeg_with(dir: &Path, name: &str, image: &RgbImage, icc: Vec<u8>, orientation: u16) -> PathBuf {
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 95);
    encoder.set_icc_profile(icc).unwrap();
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .unwrap();
    // O EXIF do ajudante de teste, logo depois do início do arquivo.
    let probe = write_jpeg(
        dir,
        "so-o-exif.jpg",
        8,
        8,
        Some(&Exif::new(
            orientation,
            "2026:09:12 15:00:40",
            "Camera de Teste",
        )),
    );
    let probe = std::fs::read(probe).unwrap();
    let app1_len = usize::from(u16::from_be_bytes([probe[4], probe[5]]));
    bytes.splice(2..2, probe[2..4 + app1_len].iter().copied());
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn dirs(root: &Path) -> (PathBuf, PathBuf) {
    (root.join("miniaturas"), root.join("telas"))
}

#[test]
fn tr_b3_cada_foto_e_lida_uma_vez_so() {
    let dir = tempfile::tempdir().unwrap();
    let exif = Exif::new(6, "2026:09:12 15:00:40", "Camera de Teste");
    let path = write_jpeg(dir.path(), "a.jpg", 600, 400, Some(&exif));
    let (fp, meta) = (fingerprint(&path).unwrap(), read_metadata(&path).unwrap());
    let bytes = std::fs::read(&path).unwrap();
    // O arquivo some depois da leitura: se o trabalho lesse de novo, falharia.
    std::fs::remove_file(&path).unwrap();
    let (thumbs, screens) = dirs(dir.path());
    let derived = DerivedDirs {
        thumbs: &thumbs,
        screens: Some(&screens),
    };
    let (entry, warning) = process_bytes(&path, bytes, (1, 2), derived).unwrap();
    assert_eq!(entry.fingerprint, fp);
    assert_eq!((entry.width, entry.height), (meta.width, meta.height));
    assert_eq!((entry.width, entry.height), (400, 600));
    assert_eq!(entry.exif_orientation, 6);
    assert_eq!((entry.taken_at, entry.camera), (meta.taken_at, meta.camera));
    assert_eq!((entry.file_size, entry.modified), (1, 2));
    assert_eq!(warning, None);
    let screen = image::open(derived_path(&screens, &fp)).unwrap();
    assert_eq!((screen.width(), screen.height()), (400, 600));
    assert_eq!(image::open(&entry.thumbnail).unwrap().height(), THUMB_SIZE);

    // O aviso do perfil ilegível sai do mesmo trabalho, sem segunda leitura (spec CT, seção 6).
    let png = dir.path().join("estranha.png");
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    encoder
        .set_icc_profile(b"nao e um perfil".to_vec())
        .unwrap();
    let picture = RgbaImage::from_pixel(30, 20, image::Rgba([90, 120, 150, 255]));
    encoder
        .write_image(picture.as_raw(), 30, 20, ExtendedColorType::Rgba8)
        .unwrap();
    let (_, warning) = process_bytes(&png, bytes, (0, 0), derived).unwrap();
    assert_eq!(
        warning.as_deref(),
        Some("A foto estranha.png tem um perfil de cor que não foi possível ler; usada como sRGB.")
    );
}

#[test]
fn tr_b5_reduzir_antes_de_converter_mantem_cores_e_orientacao() {
    let dir = tempfile::tempdir().unwrap();
    // Uma foto deitada em Adobe RGB, em pé pelo EXIF: a conversão de cor e o giro de verdade.
    let picture = crate::color::convert(
        image::DynamicImage::ImageRgb8(gradient(2400, 1600)).into_rgba8(),
        None,
        ColorSpace::AdobeRgb,
    );
    let picture = image::DynamicImage::ImageRgba8(picture).into_rgb8();
    let icc = ColorProfile::new_adobe_rgb().encode().unwrap();
    let path = jpeg_with(dir.path(), "adobe.jpg", &picture, icc, 6);
    let bytes = std::fs::read(&path).unwrap();
    let (thumbs, screens) = dirs(dir.path());
    let derived = DerivedDirs {
        thumbs: &thumbs,
        screens: Some(&screens),
    };
    let (entry, _) = process_bytes(&path, bytes, (0, 0), derived).unwrap();
    let screen = image::open(derived_path(&screens, &entry.fingerprint))
        .unwrap()
        .into_rgb8();
    // O caminho de antes: a foto inteira convertida e girada, depois reduzida.
    let before = decode_with_orientation(&path, 6)
        .unwrap()
        .thumbnail(SCREEN_SIZE, SCREEN_SIZE)
        .into_rgb8();
    assert_eq!(screen.dimensions(), before.dimensions());
    assert_eq!(screen.dimensions(), (1067, 1600), "em pé");
    // A foto como era antes de virar Adobe RGB, em pé e reduzida: as duas comparadas com ela.
    let truth = image::DynamicImage::ImageRgb8(gradient(2400, 1600))
        .rotate90()
        .thumbnail(SCREEN_SIZE, SCREEN_SIZE)
        .into_rgb8();
    // O caminho novo, na memória (sem o JPEG da imagem de tela), fica tão perto quanto o antigo.
    let (full, icc) = decoded(&path);
    let reduced = crate::derive::reduced(full, icc.as_deref(), 6, SCREEN_SIZE).into_rgb8();
    let (new, old) = (distance(&reduced, &truth), distance(&before, &truth));
    assert!(new.mean <= old.mean + 0.05, "{new:?} contra {old:?}");
    assert!(new.worst <= old.worst, "{new:?} contra {old:?}");
    // E a imagem de tela gravada (JPEG de qualidade 90) fica abaixo de 1 nível na média.
    let saved = distance(&screen, &truth);
    assert!(saved.mean < 1.0, "{saved:?}");
    // A miniatura é a imagem de tela reduzida (a mesma na janela e na linha de comando).
    let thumb = image::open(&entry.thumbnail).unwrap();
    assert_eq!((thumb.width(), thumb.height()), (213, THUMB_SIZE));
}

/// A foto inteira como o decodificador entrega, e o perfil dela.
fn decoded(path: &Path) -> (image::DynamicImage, Option<Vec<u8>>) {
    let mut decoder = image::ImageReader::open(path)
        .unwrap()
        .with_guessed_format()
        .unwrap()
        .into_decoder()
        .unwrap();
    let icc = image::ImageDecoder::icc_profile(&mut decoder).unwrap();
    (image::DynamicImage::from_decoder(decoder).unwrap(), icc)
}

/// Quanto uma imagem se afasta de outra, nível a nível.
#[derive(Debug)]
struct Distance {
    worst: u8,
    mean: f64,
}

fn distance(a: &RgbImage, b: &RgbImage) -> Distance {
    let diffs: Vec<u8> = a
        .as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| x.abs_diff(*y))
        .collect();
    let total: f64 = diffs.iter().map(|&d| f64::from(d)).sum();
    Distance {
        worst: diffs.iter().copied().max().unwrap_or(0),
        mean: total / f64::from(u32::try_from(diffs.len()).unwrap()),
    }
}
