//! Cenários da spec `opacidade-da-foto` na exportação: JPEG, TIFF e PDF do mesmo raster opaco, o
//! PDF sem transparência e os mesmos bytes a cada exportação (OP-B6 e OP-B7).

use lamina_core::{Border, Mm, Opacity, Rgb};

use super::ex::{Album, solid, spec};
use crate::{ExportFormat, ExportOptions};

/// Uma foto a `percent` sobre um fundo colorido, mais as fotos de `extra` (opacidade, borda e
/// cantos).
fn album(percent: u8, extra: &[(u8, f64, f64)]) -> Album {
    let mut album = Album::new(spec(), 1);
    album.project.spreads[0].background = Rgb {
        r: 20,
        g: 60,
        b: 240,
    };
    let a = album.photo("a.jpg", solid(600, 400, [200, 100, 40]));
    album.photo_frame(0, a, [0.1, 0.2, 0.3, 0.5]);
    album.project.spreads[0].frames[0].opacity = Opacity::new(percent).unwrap();
    for (i, (p, border, radius)) in extra.iter().enumerate() {
        let photo = album.photo(&format!("b{i}.jpg"), solid(400, 400, [40, 180, 90]));
        #[allow(clippy::cast_precision_loss)]
        let x = 0.45 + 0.15 * i as f64;
        album.photo_frame(0, photo, [x, 0.3, 0.12, 0.4]);
        let f = album.project.spreads[0].frames.last_mut().unwrap();
        f.opacity = Opacity::new(*p).unwrap();
        f.border = Border {
            width: Mm(*border),
            color: Rgb::WHITE,
        };
        f.radius = Mm(*radius);
    }
    album
}

fn export(album: &Album, format: ExportFormat) -> Vec<Vec<u8>> {
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        format,
        ..ExportOptions::default()
    };
    let report = album.export(dir.path(), &options).unwrap();
    report
        .files
        .iter()
        .map(|f| std::fs::read(f).unwrap())
        .collect()
}

fn has(bytes: &[u8], what: &[u8]) -> bool {
    bytes.windows(what.len()).any(|w| w == what)
}

#[test]
fn op_b6_jpeg_tiff_e_pdf_do_mesmo_raster() {
    let album = album(60, &[]);
    let raster = album.print(0);
    assert!(raster.pixels().all(|p| p.0[3] == 255), "o raster é opaco");
    let tiff = &export(&album, ExportFormat::Tiff)[0];
    let decoded = image::load_from_memory(tiff).unwrap().into_rgb8();
    let rgb = image::DynamicImage::ImageRgba8(raster).into_rgb8();
    assert!(
        decoded == rgb,
        "o TIFF tem os pixels exatos da renderização"
    );
    let jpeg = &export(&album, ExportFormat::Jpeg)[0];
    let pdf = &export(&album, ExportFormat::Pdf)[0];
    assert!(has(pdf, jpeg), "a imagem do PDF é o JPEG (PD-B3)");
    let doc = lopdf::Document::load_mem(pdf).unwrap();
    for object in doc.objects.values() {
        let dict = match object {
            lopdf::Object::Dictionary(d) => d,
            lopdf::Object::Stream(s) => &s.dict,
            _ => continue,
        };
        for key in [&b"SMask"[..], b"Group", b"BM", b"CA", b"ca"] {
            assert!(
                !dict.has(key),
                "o PDF não tem /{}",
                String::from_utf8_lossy(key)
            );
        }
    }
}

#[test]
fn op_b7_exportar_duas_vezes_da_o_mesmo_arquivo() {
    let album = album(30, &[(60, 1.0, 3.0), (100, 2.0, 5.0)]);
    for format in [ExportFormat::Jpeg, ExportFormat::Tiff, ExportFormat::Pdf] {
        assert_eq!(
            export(&album, format),
            export(&album, format),
            "{format:?}: os mesmos bytes"
        );
    }
}

#[test]
fn op_b14_exportar_com_opacidade_rapido() {
    use lamina_core::testing::{HEAVY_TRIES, budget};
    use lamina_core::{AlbumSpec, Dpi};
    let print = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut album = Album::new(print, 1);
    for i in 0..6u32 {
        let photo = album.photo(&format!("g{i}.jpg"), solid(6000, 4000, [120, 90, 60]));
        let (col, row) = (f64::from(i / 3), f64::from(i % 3));
        album.photo_frame(0, photo, [0.03 + col * 0.5, 0.04 + row * 0.32, 0.44, 0.29]);
        let f = album.project.spreads[0].frames.last_mut().unwrap();
        f.opacity = Opacity::new(50).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        overwrite: true,
        ..ExportOptions::default()
    };
    let mut elapsed = std::time::Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let start = std::time::Instant::now();
        album.export(dir.path(), &options).unwrap();
        elapsed = elapsed.min(start.elapsed());
        if elapsed < budget(3000) {
            break;
        }
    }
    assert!(elapsed < budget(3000), "exportar levou {elapsed:?}");
}
