//! Cenários da spec `leveza` (LV) na importação: a leitura reduzida do JPEG.

#![allow(clippy::unwrap_used)]

use std::time::Instant;

use image::GenericImageView;
use lamina_core::testing::{HEAVY_TRIES, budget};

use crate::derive::{jpeg_reduced, read_pixels};
use crate::testutil::write_jpeg;
use crate::{DerivedDirs, SCREEN_SIZE, THUMB_SIZE, derived_path, ensure_derived};

#[test]
fn lv_b13_jpeg_grande_sai_reduzido_da_leitura() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_jpeg(dir.path(), "g.jpg", 6000, 4000, None);
    let bytes = std::fs::read(&file).unwrap();
    let read = jpeg_reduced(&bytes, SCREEN_SIZE).unwrap();
    // 1/2 daria 3000, 1/4 daria 1500 (pequeno demais): fica em 1/2.
    assert_eq!(read.dimensions(), (3000, 2000));
    let (thumbs, screens) = (dir.path().join("t"), dir.path().join("s"));
    let dirs = DerivedDirs {
        thumbs: &thumbs,
        screens: Some(&screens),
    };
    ensure_derived(&file, "abc", 1, dirs).unwrap();
    let screen = image::open(derived_path(&screens, "abc")).unwrap();
    assert_eq!(screen.dimensions(), (SCREEN_SIZE, 1067));
    let thumb = image::open(derived_path(&thumbs, "abc")).unwrap();
    assert_eq!(thumb.dimensions(), (THUMB_SIZE, 213));
}

#[test]
fn lv_b13_a_reducao_mantem_as_cores() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_jpeg(dir.path(), "g.jpg", 4000, 3000, None);
    let bytes = std::fs::read(&file).unwrap();
    let reduced = jpeg_reduced(&bytes, SCREEN_SIZE).unwrap().to_rgb8();
    let full = image::load_from_memory(&bytes).unwrap().to_rgb8();
    // O canal azul do arquivo de teste é 128 em toda parte; a leitura reduzida não pode desviar.
    let (w, h) = reduced.dimensions();
    let at = |x, y| reduced.get_pixel(x, y).0;
    for (x, y) in [(0, 0), (w / 2, h / 2), (w - 1, h - 1)] {
        assert!(at(x, y)[2].abs_diff(full.get_pixel(x * 2, y * 2).0[2]) <= 3);
    }
}

#[test]
fn lv_b14_rapido_leitura_reduzida_de_24_megapixels() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_jpeg(dir.path(), "g.jpg", 6000, 4000, None);
    let mut best = std::time::Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let bytes = std::fs::read(&file).unwrap();
        let start = Instant::now();
        let image = read_pixels(&file, bytes, SCREEN_SIZE).unwrap();
        best = best.min(start.elapsed());
        assert_eq!(image.dimensions(), (3000, 2000));
        if best < budget(150) {
            break;
        }
    }
    assert!(best < budget(150), "levou {best:?}");
}

#[test]
fn lv_b15_foto_pequena_e_png_sao_lidas_inteiras() {
    let dir = tempfile::tempdir().unwrap();
    let small = write_jpeg(dir.path(), "p.jpg", 1200, 800, None);
    let bytes = std::fs::read(&small).unwrap();
    assert!(jpeg_reduced(&bytes, SCREEN_SIZE).is_none());
    assert_eq!(
        read_pixels(&small, bytes, SCREEN_SIZE)
            .unwrap()
            .dimensions(),
        (1200, 800)
    );
    let png = dir.path().join("g.png");
    image::RgbImage::new(4000, 3000).save(&png).unwrap();
    let bytes = std::fs::read(&png).unwrap();
    assert!(jpeg_reduced(&bytes, SCREEN_SIZE).is_none());
    assert_eq!(
        read_pixels(&png, bytes, SCREEN_SIZE).unwrap().dimensions(),
        (4000, 3000)
    );
}

#[test]
fn lv_b16_copias_antigas_nao_se_misturam() {
    let dir = tempfile::tempdir().unwrap();
    let name = derived_path(dir.path(), "abc");
    assert!(name.to_string_lossy().ends_with("abc.v3.jpg"), "{name:?}");
}

#[test]
fn recorte_fechado_usa_a_imagem_de_tela_antes_do_original() {
    use std::collections::HashMap;

    use lamina_core::{AlbumSpec, Dpi, Mm, Project};

    use crate::cached_source;

    let dir = tempfile::tempdir().unwrap();
    let screen = write_jpeg(dir.path(), "s.jpg", 800, 600, None);
    let spec =
        AlbumSpec::new(Mm::from_cm(20.0), Mm::from_cm(20.0), Dpi::new(100).unwrap()).unwrap();
    let mut project = Project::new("t", spec);
    // O original não existe: se a fonte o abrisse, daria erro.
    let id = project.add_photo(dir.path().join("sumiu.jpg"), 6000, 4000);
    let source = cached_source(HashMap::new(), HashMap::from([(id, screen)]));
    let photo = project.photo(id).unwrap();
    let image = source(photo, (1000, 700)).unwrap();
    assert_eq!(image.dimensions(), (800, 600));
    assert!(!source(photo, (3000, 2000)).unwrap_err().is_empty());
}
