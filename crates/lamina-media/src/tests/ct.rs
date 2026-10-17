//! Cenários da spec `cor-e-tiff` (CT) na leitura das fotos: B1, B6 e B7.

use std::path::{Path, PathBuf};

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use lamina_core::ColorSpace;
use moxcms::ColorProfile;

use crate::thumbnail::{THUMB_SIZE, make_thumbnail};
use crate::{
    Catalog, decode_with_orientation, derived_path, ensure_thumbnail, import_folder, load_oriented,
    load_oriented_in,
};

/// Degradê com cores variadas (sem extremos saturados, que saem do gamute na volta).
fn picture() -> RgbaImage {
    RgbaImage::from_fn(96, 64, |x, y| {
        let v = |n: u32| u8::try_from(30 + n % 190).unwrap();
        image::Rgba([v(x * 2), v(y * 3), v(x + y), 255])
    })
}

/// Os pixels `image` (sRGB) convertidos para Adobe RGB.
fn to_adobe(image: &RgbaImage) -> RgbaImage {
    crate::color::convert(image.clone(), None, ColorSpace::AdobeRgb)
}

/// Grava um PNG com o perfil `icc` embutido (nenhum = sem perfil).
fn write_png(dir: &Path, name: &str, image: &RgbaImage, icc: Option<Vec<u8>>) -> PathBuf {
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = PngEncoder::new(file);
    if let Some(icc) = icc {
        encoder.set_icc_profile(icc).unwrap();
    }
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .unwrap();
    path
}

/// Diferença aceita entre uma foto sRGB e a mesma foto passada por um arquivo Adobe RGB de 8 bits:
/// o LittleCMS dá a mesma (cada nível do Adobe RGB é um degrau maior; spec CT, seção 13).
const ROUND_TRIP: u8 = 4;

fn max_diff(a: &RgbaImage, b: &RgbaImage) -> u8 {
    a.as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap()
}

fn adobe_photo(dir: &Path) -> (PathBuf, PathBuf) {
    let picture = picture();
    let srgb = write_png(dir, "srgb.png", &picture, None);
    let icc = ColorProfile::new_adobe_rgb().encode().unwrap();
    let adobe = write_png(dir, "adobe.png", &to_adobe(&picture), Some(icc));
    (srgb, adobe)
}

#[test]
fn ct_b1_foto_em_adobe_rgb_com_as_cores_certas() {
    let dir = tempfile::tempdir().unwrap();
    let (srgb, adobe) = adobe_photo(dir.path());
    let (a, b) = (
        load_oriented(&srgb).unwrap(),
        load_oriented(&adobe).unwrap(),
    );
    assert!(max_diff(&a, &b) <= ROUND_TRIP, "tela: {}", max_diff(&a, &b));
    let print = load_oriented_in(&adobe, ColorSpace::Srgb).unwrap();
    assert!(max_diff(&a, &print) <= ROUND_TRIP);
    // Sem perfil, em sRGB: os mesmos pixels que o decodificador entrega.
    let raw = image::open(&srgb).unwrap().into_rgba8();
    assert_eq!(a, raw);
    // Em Adobe RGB, a foto em Adobe RGB passa sem perder.
    let kept = load_oriented_in(&adobe, ColorSpace::AdobeRgb).unwrap();
    assert!(max_diff(&kept, &to_adobe(&picture())) <= 2);
}

#[test]
fn ct_b6_miniatura_com_as_cores_certas_e_cache_novo() {
    let dir = tempfile::tempdir().unwrap();
    let (srgb, adobe) = adobe_photo(dir.path());
    let a = make_thumbnail(&srgb, 1, THUMB_SIZE).unwrap().into_rgba8();
    let b = make_thumbnail(&adobe, 1, THUMB_SIZE).unwrap().into_rgba8();
    assert!(max_diff(&a, &b) <= ROUND_TRIP, "{}", max_diff(&a, &b));
    let screen = decode_with_orientation(&adobe, 1).unwrap().into_rgba8();
    assert!(max_diff(&screen, &load_oriented(&srgb).unwrap()) <= ROUND_TRIP);
    // A miniatura de uma versão anterior do cache é ignorada e refeita.
    let thumbs = dir.path().join("miniaturas");
    std::fs::create_dir_all(&thumbs).unwrap();
    std::fs::write(thumbs.join("abc.jpg"), b"antiga").unwrap();
    let made = ensure_thumbnail(&adobe, "abc", 1, &thumbs).unwrap();
    assert_eq!(made, derived_path(&thumbs, "abc"));
    assert_ne!(made, thumbs.join("abc.jpg"));
    assert!(image::open(&made).is_ok_and(|i| i.width() > 0));
}

#[test]
fn ct_b7_perfil_que_nao_le() {
    let dir = tempfile::tempdir().unwrap();
    let photos = dir.path().join("fotos");
    std::fs::create_dir_all(&photos).unwrap();
    let picture = picture();
    let path = write_png(
        &photos,
        "estranha.png",
        &picture,
        Some(b"nao e um perfil".to_vec()),
    );
    // Usada como sRGB: os pixels de sempre.
    assert_eq!(
        load_oriented(&path).unwrap(),
        image::open(&path).unwrap().into_rgba8()
    );
    let catalog = Catalog::open_in_memory().unwrap();
    let report = import_folder(&photos, &catalog, &dir.path().join("mini")).unwrap();
    assert_eq!(report.entries.len(), 1);
    assert_eq!(
        report.warnings,
        ["A foto estranha.png tem um perfil de cor que não foi possível ler; usada como sRGB."]
    );
}
