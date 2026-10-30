//! LV-B1 (spec `leveza`): a exportação de um álbum de referência continua idêntica byte a byte
//! depois das melhorias de memória e velocidade.

use lamina_core::{AlbumSpec, ColorSpace, Dpi, FrameContent, Mm, PhotoAdjust, Rgb, TextBox};

use super::ex::Album;
use crate::{ExportFormat, ExportOptions};

/// Um gradiente com grão, sem compressão fácil: parecido com uma foto.
fn photo(w: u32, h: u32, seed: u32) -> image::RgbaImage {
    image::RgbaImage::from_fn(w, h, |x, y| {
        let grain = (x.wrapping_mul(31) ^ y.wrapping_mul(17) ^ seed) % 16;
        let channel = |v: u32| u8::try_from((v + grain).min(255)).unwrap_or(255);
        image::Rgba([
            channel(x * 200 / w),
            channel(y * 200 / h),
            channel(60 + seed * 40),
            255,
        ])
    })
}

/// Três lâminas: fotos repetidas, uma foto ajustada, uma virada no recorte e um texto.
fn reference() -> Album {
    let spec = AlbumSpec::new(Mm::from_cm(20.0), Mm::from_cm(20.0), Dpi::new(120).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut album = Album::new(spec, 3);
    let first = album.photo("a.jpg", photo(1500, 1000, 0));
    let second = album.photo("b.jpg", photo(1000, 1500, 1));
    let third = album.photo("c.jpg", photo(900, 900, 2));
    album.project.photos[1].adjust = PhotoAdjust {
        angle_tenths: 35,
        brightness: 12,
        contrast: -8,
        ..PhotoAdjust::default()
    };
    album.photo_frame(0, first, [0.03, 0.05, 0.45, 0.9]);
    album.photo_frame(0, second, [0.52, 0.05, 0.45, 0.9]);
    album.photo_frame(1, first, [0.05, 0.05, 0.4, 0.4]);
    album.photo_frame(1, third, [0.5, 0.05, 0.4, 0.4]);
    album.photo_frame(2, second, [0.1, 0.1, 0.8, 0.8]);
    let text = FrameContent::Text(TextBox {
        text: "Ana & Pedro".to_owned(),
        size_pt: 24.0,
        ..TextBox::new(Rgb::BLACK)
    });
    album.frame(2, text, [0.2, 0.4, 0.6, 0.2]);
    album
}

/// O resumo de cada arquivo exportado, em ordem de nome.
fn digests(format: ExportFormat) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        format,
        space: ColorSpace::Srgb,
        ..ExportOptions::default()
    };
    reference().export(dir.path(), &options).unwrap();
    let mut files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    files
        .iter()
        .map(|file| {
            // Os bytes do TIFF dependem do motor de Deflate que a compilação escolhe (muda entre
            // `-p lamina-export` e o workspace); o que importa são os pixels, sem perda.
            let bytes = if matches!(format, ExportFormat::Tiff) {
                image::open(file).unwrap().to_rgb8().into_raw()
            } else {
                std::fs::read(file).unwrap()
            };
            blake3::hash(&bytes).to_hex()[..16].to_owned()
        })
        .collect()
}

/// Gravado na base (beta 7, antes da spec LV). As fotos podem variar ±1 nível noutros
/// processadores (regra 12), por isso a comparação fixa só vale no Windows x86-64, onde a porta
/// local roda; nos outros sistemas, duas exportações seguidas têm que ser iguais.
const JPEG: [&str; 3] = ["7cbd9a199fa05295", "6ec3e255a817d7b7", "0f24a7dd57d69bff"];
const PDF: [&str; 1] = ["026d7fb04e067810"];
const TIFF: [&str; 3] = ["1bb661a9e8fcafe6", "448ff7c8343b4d5f", "d02ebffdea90b77c"];

fn check(format: ExportFormat, golden: &[&str]) {
    let now = digests(format);
    if cfg!(all(windows, target_arch = "x86_64")) {
        assert_eq!(now, golden, "a exportação mudou");
    } else {
        assert_eq!(now, digests(format));
    }
}

#[test]
fn lv_b1_exportacao_identica_em_jpeg_pdf_e_tiff() {
    check(ExportFormat::Jpeg, &JPEG);
    check(ExportFormat::Pdf, &PDF);
    check(ExportFormat::Tiff, &TIFF);
}

#[test]
fn lv_b20_o_codificador_jpeg_continua_o_mesmo() {
    // O codificador mais rápido não entrou: sem como provar que sai igual em qualquer processador
    // (regra 12), fica o de hoje. Os bytes iguais garantem 4:4:4, qualidade 95, DPI e perfil.
    check(ExportFormat::Jpeg, &JPEG);
}
