//! CT-B9 (spec `cor-e-tiff`): `lamina exportar --formato tiff --cor adobe-rgb`, e o erro de
//! `--cor` inválido.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, files, text};
use lamina_core::ColorSpace;
use lamina_export::color_profile;

#[test]
fn ct_b9_linha_de_comando() {
    let base = Base::with_photos(4, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--nome", "album"]);
    assert!(out.status.success(), "{}", text(&out.stderr));

    let out = base.lamina(&[
        "exportar",
        "saida/album.lamina",
        "--saida",
        "tiff",
        "--formato",
        "tiff",
        "--cor",
        "adobe-rgb",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        files(&base.path("tiff")),
        ["lamina-01.tif", "lamina-02.tif"]
    );
    let bytes = std::fs::read(base.path("tiff/lamina-01.tif")).unwrap();
    let mut decoder = tiff::decoder::Decoder::new(std::io::Cursor::new(&bytes)).unwrap();
    let icc = decoder.get_tag_u8_vec(tiff::tags::Tag::IccProfile).unwrap();
    assert_eq!(icc, color_profile(ColorSpace::AdobeRgb).unwrap());

    let bad = base.lamina(&[
        "exportar",
        "saida/album.lamina",
        "--saida",
        "x",
        "--cor",
        "adobe",
    ]);
    assert_eq!(bad.status.code(), Some(1));
    assert_eq!(
        text(&bad.stderr).trim(),
        "Erro: valor inválido para --cor: adobe."
    );
}
