//! Cenário JE-B1 da spec `janela-enxuta`, no arquivo exportado: o X do quadro vazio é só da tela.

use lamina_core::FrameContent;

use super::ex::{Album, solid, spec};
use crate::ExportOptions;

/// Uma lâmina com duas fotos e, se `empty`, um quadro vazio.
fn album(empty: bool) -> Album {
    let mut album = Album::new(spec(), 1);
    let a = album.photo("a.jpg", solid(600, 400, [200, 50, 50]));
    let b = album.photo("b.jpg", solid(600, 400, [40, 90, 200]));
    album.photo_frame(0, a, [0.05, 0.2, 0.2, 0.4]);
    album.photo_frame(0, b, [0.3, 0.2, 0.2, 0.4]);
    if empty {
        album.frame(0, FrameContent::Empty, [0.6, 0.2, 0.3, 0.6]);
    }
    album
}

/// Os bytes de cada arquivo da exportação em JPEG.
fn export(album: &Album) -> Vec<Vec<u8>> {
    let dir = tempfile::tempdir().unwrap();
    let report = album.export(dir.path(), &ExportOptions::default()).unwrap();
    report
        .files
        .iter()
        .map(|f| std::fs::read(f).unwrap())
        .collect()
}

#[test]
fn je_b1_o_x_e_so_da_tela() {
    let with = album(true);
    let without = album(false);
    assert_eq!(
        with.print(0).as_raw(),
        without.print(0).as_raw(),
        "a imagem de impressão é a da lâmina sem o quadro vazio"
    );
    let first = export(&with);
    assert_eq!(first, export(&with), "exportar de novo dá os mesmos bytes");
    assert_eq!(first, export(&without), "o arquivo não tem o X nem o cinza");
}
