//! TR-B1 (spec `travamento-com-muitas-fotos`): o pico de memória da importação, medido pelo
//! `dhat`. Fica num binário de teste só dele, porque o `dhat` troca o alocador do programa.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbImage};
use lamina_media::{Catalog, DerivedDirs, import_folder_with, photos_at_once};
use moxcms::ColorProfile;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const MIB: u64 = 1024 * 1024;
/// Fotos de 12 MP: grandes o bastante para o pico ser delas, e o teste rápido.
const SIZE: (u32, u32) = (4000, 3000);

/// Um JPEG `SIZE` com o perfil `icc` e o EXIF com a orientação `orientation` (1 ou 6).
fn base(icc: Vec<u8>, orientation: u8) -> Vec<u8> {
    let (w, h) = SIZE;
    let image = RgbImage::from_fn(w, h, |x, y| {
        let v = |n: u32| u8::try_from(30 + n % 190).unwrap();
        image::Rgb([v(x / 21), v(y / 16), v((x ^ y) % 97)])
    });
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 90);
    encoder.set_icc_profile(icc).unwrap();
    encoder
        .write_image(image.as_raw(), w, h, ExtendedColorType::Rgb8)
        .unwrap();
    // EXIF mínimo (TIFF little-endian, uma entrada: a orientação), logo depois do início.
    let mut tiff = b"II\x2a\x00\x08\x00\x00\x00\x01\x00\x12\x01\x03\x00\x01\x00\x00\x00".to_vec();
    tiff.extend([orientation, 0, 0, 0, 0, 0, 0, 0]);
    let mut app1 = vec![0xFF, 0xE1];
    app1.extend(u16::try_from(2 + 6 + tiff.len()).unwrap().to_be_bytes());
    app1.extend(b"Exif\0\0");
    app1.extend(tiff);
    bytes.splice(2..2, app1);
    bytes
}

/// A mesma foto com um comentário próprio logo depois do início: outra impressão digital, os
/// mesmos pixels.
fn copy(dir: &Path, name: &str, bytes: &[u8], n: usize) -> u64 {
    let text = format!("copia {n}");
    let mut out = bytes[..2].to_vec();
    out.extend([0xFF, 0xFE]);
    out.extend(u16::try_from(text.len() + 2).unwrap().to_be_bytes());
    out.extend(text.as_bytes());
    out.extend(&bytes[2..]);
    std::fs::write(dir.join(name), &out).unwrap();
    u64::try_from(out.len()).unwrap()
}

#[test]
fn tr_b1_importar_fotos_grandes_tem_pico_de_memoria_contado() {
    let dir = tempfile::tempdir().unwrap();
    let photos = dir.path().join("fotos");
    std::fs::create_dir_all(&photos).unwrap();
    // Metade como sai do Lightroom (perfil sRGB), metade em Adobe RGB e de pé pelo EXIF: os
    // caminhos que mais pesavam (conversão de cor e giro da foto inteira).
    let srgb = base(ColorProfile::new_srgb().encode().unwrap(), 1);
    let adobe = base(ColorProfile::new_adobe_rgb().encode().unwrap(), 6);
    let mut largest = 0;
    for n in 0..8 {
        let bytes = if n % 2 == 0 { &srgb } else { &adobe };
        largest = largest.max(copy(&photos, &format!("{n}.jpg"), bytes, n));
    }
    let catalog = Catalog::open_in_memory().unwrap();
    let (thumbs, screens) = (dir.path().join("miniaturas"), dir.path().join("telas"));
    let dirs = DerivedDirs {
        thumbs: &thumbs,
        screens: Some(&screens),
    };

    let profiler = dhat::Profiler::builder()
        .testing()
        .trim_backtraces(Some(4))
        .build();
    let report = import_folder_with(&photos, &catalog, dirs, &|_, _| {}).unwrap();
    let peak = u64::try_from(dhat::HeapStats::get().max_bytes).unwrap();
    drop(profiler);

    assert_eq!((report.processed, report.failures.len()), (8, 0));
    // Por foto em andamento: a decodificada em RGB (3 bytes por pixel) com folga até 4, o arquivo
    // duas vezes (o decodificador copia) e as cópias reduzidas. Antes eram 7 a 8 bytes por pixel
    // por núcleo (RGBA, conversão de cor e giro na foto inteira).
    let pixels = u64::from(SIZE.0) * u64::from(SIZE.1);
    let per_photo = 4 * pixels + 2 * largest + 16 * MIB;
    let at_once = u64::try_from(photos_at_once()).unwrap();
    let limit = at_once * per_photo + 24 * MIB;
    eprintln!(
        "TR-B1: pico {} MiB, limite {} MiB ({at_once} fotos ao mesmo tempo)",
        peak / MIB,
        limit / MIB
    );
    assert!(
        peak <= limit,
        "pico de {} MiB passou do limite de {} MiB",
        peak / MIB,
        limit / MIB
    );
}
