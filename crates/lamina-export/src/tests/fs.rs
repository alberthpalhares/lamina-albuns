//! Cenários da spec `fontes-do-computador` (FS) na exportação: o aviso da fonte que falta (B6) e
//! os mesmos arquivos na mesma máquina (B10).

use lamina_core::{AlbumSpec, ColorSpace, Dpi, FrameContent, Mm, Rgb, TextBox};
use lamina_render::FAMILIES;
use lamina_render::testing::{FONT_A, TestFonts};

use super::ex::{Album, solid};
use crate::{ExportFormat, ExportOptions, preflight};

fn text(s: &str, font: &str) -> FrameContent {
    FrameContent::Text(TextBox {
        text: s.to_owned(),
        font: font.to_owned(),
        ..TextBox::new(Rgb::BLACK)
    })
}

/// Uma lâmina pequena com uma foto e um texto em cada fonte de `fonts`.
fn album(fonts: &[&str]) -> Album {
    let spec = AlbumSpec::new(Mm::from_cm(10.0), Mm::from_cm(10.0), Dpi::new(100).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut a = Album::new(spec, 1);
    let p = a.photo("f.jpg", solid(600, 400, [90, 140, 200]));
    a.photo_frame(0, p, [0.05, 0.1, 0.4, 0.8]);
    for (i, font) in fonts.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let y = 0.05 + 0.12 * i as f64;
        a.frame(0, text("Ana & Pedro, ação", font), [0.55, y, 0.4, 0.1]);
    }
    a
}

#[test]
fn fs_b6_a_pre_impressao_avisa_a_fonte_que_nao_esta_neste_computador() {
    let _fonts = TestFonts::standard();
    let messages: Vec<String> = preflight(&album(&["Lobster", FONT_A]).project, &[0])
        .iter()
        .map(ToString::to_string)
        .filter(|m| m.contains("fonte"))
        .collect();
    // Só a "Lobster" falta: a "Fonte A" está instalada.
    assert_eq!(
        messages,
        ["Lâmina 1: a fonte \"Lobster\" não está neste computador; o texto usa Inter."]
    );
}

#[test]
fn fs_b10_mesma_maquina_mesmos_arquivos() {
    let _fonts = TestFonts::standard();
    let mut fonts: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
    fonts.push(FONT_A);
    let a = album(&fonts);
    for format in [ExportFormat::Jpeg, ExportFormat::Pdf, ExportFormat::Tiff] {
        let options = ExportOptions {
            format,
            space: ColorSpace::AdobeRgb,
            ..ExportOptions::default()
        };
        let (one, two) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let first = a.export(one.path(), &options).unwrap();
        a.export(two.path(), &options).unwrap();
        for file in &first.files {
            let name = file.file_name().unwrap();
            let (x, y) = (
                std::fs::read(one.path().join(name)).unwrap(),
                std::fs::read(two.path().join(name)).unwrap(),
            );
            assert!(x == y, "{format:?}: {name:?} mudou");
        }
    }
}
