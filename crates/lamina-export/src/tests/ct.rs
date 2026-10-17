//! Cenários da spec `cor-e-tiff` (CT) na exportação: B3, B4, B5, B8 e B10. A leitura das fotos
//! com perfil (B1, B6, B7) mora no `lamina-media`; B2 lê fotos de verdade pelo editor; B9 roda o
//! binário `lamina`.

use std::path::Path;

use image::{ImageDecoder, Rgba, RgbaImage};
use lamina_core::testing::{HEAVY_TRIES, budget};
use lamina_core::{AlbumSpec, ColorSpace, Dpi, Mm, Photo, Rgb};
use lamina_render::{RenderTarget, render_spread};
use lopdf::Document;

use super::ex::{Album, solid};
use crate::{ExportFormat, ExportOptions, srgb_profile};

fn spec(dpi: u32) -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(10.0), Mm::from_cm(10.0), Dpi::new(dpi).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap()
}

fn album(spreads: usize) -> Album {
    let mut album = Album::new(spec(100), spreads);
    for s in 0..spreads {
        let tone = u8::try_from(40 + s * 50).unwrap();
        let p = album.photo(&format!("f{s}.jpg"), solid(60, 40, [tone, 120, 200 - tone]));
        album.photo_frame(s, p, [0.1, 0.2, 0.8, 0.6]);
    }
    album
}

fn options(format: ExportFormat, space: ColorSpace) -> ExportOptions {
    ExportOptions {
        format,
        space,
        ..ExportOptions::default()
    }
}

fn icc_of_jpeg(path: &Path) -> Vec<u8> {
    let file = std::io::BufReader::new(std::fs::File::open(path).unwrap());
    let mut decoder = image::codecs::jpeg::JpegDecoder::new(file).unwrap();
    decoder.icc_profile().unwrap().unwrap()
}

/// A descrição do perfil aparece no texto dele (a etiqueta `desc`).
fn mentions(icc: &[u8], text: &str) -> bool {
    let utf16: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    icc.windows(text.len()).any(|w| w == text.as_bytes())
        || icc.windows(utf16.len()).any(|w| w == utf16.as_slice())
}

#[test]
fn ct_b3_fundo_em_adobe_rgb() {
    let mut album = album(1);
    album.project.spreads[0].background = Rgb { r: 255, g: 0, b: 0 };
    let dir = tempfile::tempdir().unwrap();
    let opts = ExportOptions {
        quality: 100,
        ..options(ExportFormat::Jpeg, ColorSpace::AdobeRgb)
    };
    album.export(dir.path(), &opts).unwrap();
    let file = dir.path().join("lamina-01.jpg");
    let image = image::open(&file).unwrap().to_rgb8();
    let corner = image.get_pixel(2, 2);
    assert!((i32::from(corner[0]) - 219).abs() <= 2, "{corner:?}");
    assert!(corner[1] <= 3 && corner[2] <= 3, "{corner:?}");
    assert!(mentions(&icc_of_jpeg(&file), "Adobe RGB (1998)"));
}

#[test]
fn ct_b4_tiff_sem_perda() {
    let album = album(3);
    let dir = tempfile::tempdir().unwrap();
    let report = album
        .export(dir.path(), &options(ExportFormat::Tiff, ColorSpace::Srgb))
        .unwrap();
    let names: Vec<String> = report
        .files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["lamina-01.tif", "lamina-02.tif", "lamina-03.tif"]);
    let source = |p: &Photo, _: (u32, u32)| {
        album
            .images
            .get(&p.path)
            .cloned()
            .ok_or_else(|| "sumiu".to_owned())
    };
    for (i, file) in report.files.iter().enumerate() {
        let rendered = render_spread(&album.project, i, RenderTarget::Print, &source).unwrap();
        let rgb = image::DynamicImage::ImageRgba8(rendered).into_rgb8();
        let tiff = image::open(file).unwrap().into_rgb8();
        assert!(tiff == rgb, "{file:?}: pixels diferentes da renderização");
        let bytes = std::fs::read(file).unwrap();
        assert!(bytes.len() < rgb.as_raw().len(), "comprimido");
        let mut decoder = tiff::decoder::Decoder::new(std::io::Cursor::new(&bytes)).unwrap();
        let icc = decoder.get_tag_u8_vec(tiff::tags::Tag::IccProfile).unwrap();
        assert_eq!(icc, srgb_profile().unwrap());
        let compression = decoder.get_tag_u32(tiff::tags::Tag::Compression).unwrap();
        assert_eq!(compression, 8, "Deflate");
        let x = decoder
            .get_tag_u32_vec(tiff::tags::Tag::XResolution)
            .unwrap();
        assert_eq!(x, [100, 1], "DPI da especificação");
    }
}

#[test]
fn ct_b5_pdf_em_adobe_rgb() {
    let dir = tempfile::tempdir().unwrap();
    album(1)
        .export(
            dir.path(),
            &options(ExportFormat::Pdf, ColorSpace::AdobeRgb),
        )
        .unwrap();
    let doc = Document::load(dir.path().join("teste.pdf")).unwrap();
    let catalog = doc.catalog().unwrap();
    let intents = catalog.get(b"OutputIntents").unwrap().as_array().unwrap();
    let intent = intents[0].as_dict().unwrap();
    let id = intent.get(b"OutputConditionIdentifier").unwrap();
    assert_eq!(id.as_str().unwrap(), b"Adobe RGB (1998)");
    let profile = intent
        .get(b"DestOutputProfile")
        .unwrap()
        .as_reference()
        .unwrap();
    let icc = &doc
        .get_object(profile)
        .unwrap()
        .as_stream()
        .unwrap()
        .content;
    assert_eq!(&icc[12..16], b"prtr");
    assert_eq!(&icc[20..24], b"Lab ");
    // A imagem usa o perfil compatível com Adobe RGB.
    let page = doc.get_pages()[&1];
    let (resources, _) = doc.get_page_resources(page).unwrap();
    let xobjects = resources
        .unwrap()
        .get(b"XObject")
        .unwrap()
        .as_dict()
        .unwrap();
    let image = xobjects.get(b"Im0").unwrap().as_reference().unwrap();
    let dict = &doc.get_object(image).unwrap().as_stream().unwrap().dict;
    let space = dict.get(b"ColorSpace").unwrap().as_array().unwrap();
    let image_icc = space[1].as_reference().unwrap();
    let image_icc = &doc
        .get_object(image_icc)
        .unwrap()
        .as_stream()
        .unwrap()
        .content;
    assert_eq!(&image_icc[12..16], b"mntr");
    assert!(mentions(image_icc, "Adobe RGB (1998)"));
}

#[test]
fn ct_b8_srgb_continua_byte_a_byte() {
    let album = album(2);
    for format in [ExportFormat::Jpeg, ExportFormat::Pdf] {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let default = ExportOptions {
            format,
            ..ExportOptions::default()
        };
        album.export(a.path(), &default).unwrap();
        album
            .export(b.path(), &options(format, ColorSpace::Srgb))
            .unwrap();
        for entry in std::fs::read_dir(a.path()).unwrap() {
            let name = entry.unwrap().file_name();
            let (x, y) = (
                std::fs::read(a.path().join(&name)).unwrap(),
                std::fs::read(b.path().join(&name)).unwrap(),
            );
            assert!(x == y, "{name:?} mudou");
        }
    }
    // O JPEG em sRGB leva o mesmo perfil de sempre.
    let dir = tempfile::tempdir().unwrap();
    album.export(dir.path(), &ExportOptions::default()).unwrap();
    assert_eq!(
        icc_of_jpeg(&dir.path().join("lamina-01.jpg")),
        srgb_profile().unwrap()
    );
}

#[test]
fn ct_b10_rapido_o_bastante() {
    let print = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut album = Album::new(print, 10);
    let photos: Vec<_> = (0..6u32)
        .map(|n| {
            let mut image: RgbaImage = solid(6000, 4000, [120, 90, 60]);
            image.put_pixel(n, n, Rgba([255, 255, 255, 255]));
            album.photo(&format!("g{n}.jpg"), image)
        })
        .collect();
    for s in 0..10 {
        for (i, photo) in photos.iter().enumerate() {
            let (col, row) = (i / 3, i % 3);
            let x = 0.03 + f64::from(u32::try_from(col).unwrap()) * 0.5;
            let y = 0.04 + f64::from(u32::try_from(row).unwrap()) * 0.32;
            album.photo_frame(s, *photo, [x, y, 0.44, 0.29]);
        }
    }
    for format in [ExportFormat::Jpeg, ExportFormat::Tiff] {
        let dir = tempfile::tempdir().unwrap();
        let opts = ExportOptions {
            overwrite: true,
            ..options(format, ColorSpace::AdobeRgb)
        };
        let mut elapsed = std::time::Duration::MAX;
        for _ in 0..HEAVY_TRIES {
            let start = std::time::Instant::now();
            album.export(dir.path(), &opts).unwrap();
            elapsed = elapsed.min(start.elapsed());
            if elapsed < budget(15_000) {
                break;
            }
        }
        assert!(elapsed < budget(15_000), "{format:?}: {elapsed:?}");
    }
}
