//! AF-B15 (spec `ajustes-da-foto`): o PDF com uma foto ajustada e um texto por cima tem a mesma
//! estrutura do PDF sem ajuste, e o texto continua por cima da foto.

use lamina_core::{AlbumSpec, Dpi, FrameContent, Mm, PhotoAdjust, Rgb, TextBox};
use lopdf::Document;

use super::ex::{Album, solid};
use crate::{ExportFormat, ExportOptions};

/// Uma foto clara num quadro grande e um texto preto por cima, a 100 DPI.
fn album(adjust: PhotoAdjust) -> Album {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(100).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut album = Album::new(spec, 1);
    let photo = album.photo("clara.jpg", solid(300, 200, [230, 230, 230]));
    album.project.photos[0].adjust = adjust;
    album.photo_frame(0, photo, [0.05, 0.1, 0.4, 0.8]);
    let text = FrameContent::Text(TextBox {
        text: "MMMM".to_owned(),
        size_pt: 72.0,
        ..TextBox::new(Rgb::BLACK)
    });
    album.frame(0, text, [0.1, 0.3, 0.3, 0.3]);
    album
}

/// Os nomes dos objetos de cada página (tipo e chaves do dicionário), sem os bytes.
fn structure(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = doc
        .objects
        .values()
        .map(|o| match o {
            lopdf::Object::Dictionary(d) => {
                let mut keys: Vec<String> = d
                    .iter()
                    .map(|(k, _)| String::from_utf8_lossy(k).into())
                    .collect();
                keys.sort();
                format!("dict {}", keys.join(","))
            }
            lopdf::Object::Stream(s) => {
                let mut keys: Vec<String> = s
                    .dict
                    .iter()
                    .filter(|(k, _)| k.as_slice() != b"Length")
                    .map(|(k, _)| String::from_utf8_lossy(k).into())
                    .collect();
                keys.sort();
                format!("stream {}", keys.join(","))
            }
            other => format!("{:?}", std::mem::discriminant(other)),
        })
        .collect();
    out.sort();
    out
}

#[test]
fn af_b15_pdf_com_ajustes_mantem_a_estrutura_e_o_texto_por_cima() {
    let options = ExportOptions {
        format: ExportFormat::Pdf,
        ..ExportOptions::default()
    };
    let adjusted = album(PhotoAdjust {
        angle_tenths: 50,
        brightness: 20,
        contrast: 30,
        bw: true,
        ..PhotoAdjust::default()
    });
    let (plain_dir, adjusted_dir) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    album(PhotoAdjust::default())
        .export(plain_dir.path(), &options)
        .unwrap();
    adjusted.export(adjusted_dir.path(), &options).unwrap();
    let plain = Document::load(plain_dir.path().join("teste.pdf")).unwrap();
    let turned = Document::load(adjusted_dir.path().join("teste.pdf")).unwrap();
    assert_eq!(structure(&plain), structure(&turned));
    // O texto preto aparece por cima da foto clara, na impressão da lâmina.
    let img = adjusted.print(0);
    let canvas = adjusted.project.spec.canvas_rect();
    let rect = adjusted
        .project
        .frame_rect(&adjusted.project.spreads[0].frames[1]);
    let per = f64::from(img.width()) / canvas.width.get();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = |mm: f64| (mm * per).round() as u32;
    let dark = (px(rect.y.get())..px(rect.bottom().get()))
        .flat_map(|y| (px(rect.x.get())..px(rect.right().get())).map(move |x| (x, y)))
        .filter(|&(x, y)| img.get_pixel(x, y).0[0] < 60)
        .count();
    assert!(
        dark > 100,
        "o texto sumiu debaixo da foto ({dark} pixels escuros)"
    );
}

#[test]
fn af_b5_pre_impressao_conta_o_giro() {
    // Quadro de 300 × 200 mm a 300 DPI: a foto de 3200 px dá 271 DPI (ampliada 111%, sem aviso).
    let mut album = Album::new(super::ex::spec(), 1);
    let a = album.photo("a.jpg", solid(3200, 2133, [9, 9, 9]));
    album.photo_frame(0, a, [0.25, 1.0 / 6.0, 0.5, 2.0 / 3.0]);
    let warnings = |album: &Album| -> Vec<String> {
        crate::preflight(&album.project, &[0])
            .iter()
            .map(ToString::to_string)
            .collect()
    };
    assert!(warnings(&album).is_empty(), "{:?}", warnings(&album));
    // Girada 10°, o fator de cobertura (≈ 1,25) passa do limite: o aviso de ampliação aparece.
    album.project.photos[0].adjust.angle_tenths = 100;
    let w = warnings(&album);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with("Lâmina 1: a foto a.jpg está ampliada"),
        "{w:?}"
    );
}
