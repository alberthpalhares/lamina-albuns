//! Casos-limite do `lamina build` achados na revisão (spec CB, regra 5 e seção 6): nada é
//! gravado quando um JPEG já existe, quando uma opção é inválida ou quando o nome não serve.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, files, text};

#[test]
fn jpeg_que_ja_existe_impede_tudo() {
    let base = Base::with_photos(4, (300, 200));
    std::fs::create_dir_all(base.path("saida")).unwrap();
    std::fs::write(base.path("saida/lamina-01.jpg"), "antigo").unwrap();
    let out = base.build("saida", &["--laminas", "1"]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr).contains("Já existe lamina-01.jpg em saida"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        files(&base.path("saida")),
        ["lamina-01.jpg"],
        "o projeto não foi gravado"
    );
}

#[test]
fn opcao_invalida_com_sobrescrever_nao_grava_o_projeto() {
    let base = Base::with_photos(4, (300, 200));
    let out = base.build(
        "saida",
        &["--laminas", "1", "--sobrescrever", "--qualidade", "0"],
    );
    assert!(!out.status.success());
    assert!(text(&out.stderr).contains("A qualidade do JPEG vai de 1 a 100, não 0."));
    assert!(files(&base.path("saida")).is_empty(), "nada gravado");
}

#[test]
fn nome_invalido_e_recusado() {
    let base = Base::with_photos(4, (300, 200));
    let out = base.build("saida", &["--laminas", "1", "--nome", "c:x"]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr).contains("Nome de projeto inválido: \"c:x\""),
        "{}",
        text(&out.stderr)
    );
    assert!(files(&base.path("saida")).is_empty());
}

#[test]
fn minimo_de_paginas_zero_e_recusado() {
    let base = Base::with_photos(4, (300, 200));
    let out = base.build("saida", &["--laminas", "1", "--paginas-minimas", "0"]);
    assert!(!out.status.success());
}

/// Spec CB, seção 6: foto que não importa vira aviso e o build segue (auditoria do Marco 1, M10).
#[test]
fn foto_quebrada_vira_aviso_e_segue() {
    let base = Base::with_photos(4, (300, 200));
    std::fs::write(base.path("fotos/quebrada.jpg"), b"isto nao e uma foto").unwrap();
    let out = base.build("saida", &["--laminas", "1"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).contains("Não foi possível importar quebrada.jpg"),
        "{}",
        text(&out.stdout)
    );
    assert!(files(&base.path("saida")).contains(&"lamina-01.jpg".to_owned()));
}
