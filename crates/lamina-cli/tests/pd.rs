//! PD-B10 (spec `pdf-de-impressao`): o PDF da janela é o mesmo do `lamina exportar --formato
//! pdf`, e os erros de opção saem em português.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, text};
use lamina_export::{ExportFormat, ExportOptions, open_project};

#[test]
fn pd_b10_janela_e_linha_de_comando_geram_o_mesmo_pdf() {
    let base = Base::with_photos(6, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--nome", "album"]);
    assert!(out.status.success(), "{}", text(&out.stderr));

    let out = base.lamina(&[
        "exportar",
        "saida/album.lamina",
        "--saida",
        "linha",
        "--formato",
        "pdf",
        "--marcas-de-corte",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));

    // A janela exporta pela mesma função do editor (spec EI, regra 1).
    let project = open_project(&base.path("saida/album.lamina")).unwrap();
    let options = ExportOptions {
        format: ExportFormat::Pdf,
        crop_marks: true,
        ..ExportOptions::default()
    };
    lamina_editor::export_project(&project, &base.path("janela"), &options, &|_, _| {}).unwrap();

    let cli = std::fs::read(base.path("linha/album.pdf")).unwrap();
    let window = std::fs::read(base.path("janela/album.pdf")).unwrap();
    assert!(cli == window, "os dois PDFs diferem");
}

#[test]
fn pd_b10_erros_das_opcoes_do_pdf() {
    let base = Base::with_photos(0, (10, 10));
    let marks = base.lamina(&["exportar", "x.lamina", "--saida", "s", "--marcas-de-corte"]);
    assert_eq!(marks.status.code(), Some(1));
    assert_eq!(
        text(&marks.stderr).trim(),
        "Erro: marcas de corte só existem no PDF (use --formato pdf)."
    );
    // No build, o erro das marcas vem antes até do erro da pasta de fotos que não existe.
    let build = base.lamina(&["build", "nada", "--saida", "s", "--marcas-de-corte"]);
    assert_eq!(build.status.code(), Some(1));
    assert_eq!(
        text(&build.stderr).trim(),
        "Erro: marcas de corte só existem no PDF (use --formato pdf)."
    );
    let png = base.lamina(&["exportar", "x.lamina", "--saida", "s", "--formato", "png"]);
    assert_eq!(png.status.code(), Some(1));
    assert_eq!(
        text(&png.stderr).trim(),
        "Erro: valor inválido para --formato: png."
    );
}
