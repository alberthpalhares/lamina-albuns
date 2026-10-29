//! Cenários da spec `texto-e-quadros` (TX) na exportação: avisos (B4, B5, B14, B15) e os mesmos
//! arquivos a cada exportação (B13).

use lamina_core::{AlbumSpec, ColorSpace, Dpi, FrameContent, Mm, Rgb, TextBox};
use lamina_render::FAMILIES;

use super::ex::{Album, solid};
use crate::{ExportFormat, ExportOptions, preflight};

/// Lâmina de 2 × 300 mm por 300 mm, zona segura de 10 mm.
fn album() -> Album {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap();
    Album::new(spec, 1)
}

fn text(s: &str, font: &str) -> FrameContent {
    FrameContent::Text(TextBox {
        text: s.to_owned(),
        font: font.to_owned(),
        ..TextBox::new(Rgb::BLACK)
    })
}

/// Um retângulo em mm na lâmina de 600 × 300 mm, em frações.
fn mm(x: f64, y: f64, w: f64, h: f64) -> [f64; 4] {
    [x / 600.0, y / 300.0, w / 600.0, h / 300.0]
}

fn messages(album: &Album) -> Vec<String> {
    preflight(&album.project, &[0])
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn tx_b4_aviso_de_texto_que_nao_cabe() {
    let mut a = album();
    a.frame(
        0,
        text("Ana & Pedro\nSempre\nJuntos", "Inter"),
        mm(50.0, 50.0, 40.0, 10.0),
    );
    assert_eq!(
        messages(&a),
        ["Lâmina 1: o texto \"Ana & Pedro…\" não cabe no quadro."]
    );
}

#[test]
fn tx_b5_texto_fora_da_zona_segura_e_na_dobra() {
    let mut a = album();
    a.frame(
        0,
        text("Perto do corte", "Inter"),
        mm(3.0, 100.0, 100.0, 40.0),
    );
    a.frame(0, text("Na dobra", "Inter"), mm(250.0, 100.0, 100.0, 40.0));
    a.frame(0, text("Dentro", "Inter"), mm(400.0, 100.0, 100.0, 40.0));
    a.frame(0, text("   ", "Inter"), mm(0.0, 0.0, 50.0, 20.0));
    assert_eq!(
        messages(&a),
        [
            "Lâmina 1: o texto \"Perto do corte\" está fora da zona segura.",
            "Lâmina 1: o texto \"Na dobra\" está fora da zona segura.",
        ]
    );
}

fn small_album() -> Album {
    let spec = AlbumSpec::new(Mm::from_cm(10.0), Mm::from_cm(10.0), Dpi::new(100).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut a = Album::new(spec, 1);
    let p = a.photo("f.jpg", solid(600, 400, [90, 140, 200]));
    a.photo_frame(0, p, [0.05, 0.1, 0.4, 0.8]);
    a
}

#[test]
fn tx_b13_mesma_entrada_mesmos_arquivos() {
    let mut a = small_album();
    for (i, f) in FAMILIES.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let y = 0.05 + 0.15 * i as f64;
        a.frame(0, text("Ana & Pedro, ação", f.name), [0.55, y, 0.4, 0.12]);
    }
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

#[test]
fn tx_b14_fonte_que_nao_existe() {
    let _fonts = lamina_render::testing::TestFonts::none();
    let mut lobster = small_album();
    lobster.frame(0, text("Ana & Pedro", "Lobster"), [0.55, 0.3, 0.4, 0.3]);
    assert_eq!(
        messages(&lobster),
        ["Lâmina 1: a fonte \"Lobster\" não está neste computador; o texto usa Inter."]
    );
    // Sai igual ao mesmo texto em Inter.
    let mut inter = small_album();
    inter.frame(0, text("Ana & Pedro", "Inter"), [0.55, 0.3, 0.4, 0.3]);
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let options = ExportOptions::default();
    lobster.export(a.path(), &options).unwrap();
    inter.export(b.path(), &options).unwrap();
    let read = |d: &tempfile::TempDir| std::fs::read(d.path().join("lamina-01.jpg")).unwrap();
    assert!(read(&a) == read(&b));
}

#[test]
fn tx_b15_aviso_de_caractere_que_a_fonte_nao_tem() {
    let mut a = album();
    a.frame(
        0,
        text("Ana 🙂 Pedro", "Inter"),
        mm(50.0, 50.0, 200.0, 40.0),
    );
    assert_eq!(
        messages(&a),
        ["Lâmina 1: a fonte não tem o caractere \"🙂\"."]
    );
}
