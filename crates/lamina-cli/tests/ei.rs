//! Cenários da spec `exportar-pela-janela` (EI) no binário `lamina`: a limpeza da saída no
//! `--sobrescrever` (B4) e os erros dos argumentos em português (B11).

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, files, text};

#[test]
fn ei_b4_exportar_com_sobrescrever_apaga_so_os_que_sobraram() {
    let base = Base::with_photos(6, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--nome", "album"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    std::fs::write(base.path("saida/lamina-09.jpg"), "antigo").unwrap();
    std::fs::write(base.path("saida/notas.txt"), "minhas notas").unwrap();

    let out = base.lamina(&[
        "exportar",
        "saida/album.lamina",
        "--saida",
        "saida",
        "--sobrescrever",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        files(&base.path("saida")),
        [
            "album.lamina",
            "lamina-01.jpg",
            "lamina-02.jpg",
            "notas.txt"
        ]
    );
    assert!(
        text(&out.stdout).contains("1 arquivo antigo apagado"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn ei_b11_erros_dos_argumentos_em_portugues() {
    let base = Base::with_photos(0, (10, 10));
    let unknown = base.lamina(&["build", "fotos", "--saida", "s", "--laminass", "3"]);
    assert_eq!(unknown.status.code(), Some(1));
    assert_eq!(
        text(&unknown.stderr).trim(),
        "Erro: opção desconhecida: --laminass."
    );
    let bad = base.lamina(&["build", "fotos", "--saida", "s", "--laminas", "tres"]);
    assert_eq!(bad.status.code(), Some(1));
    assert_eq!(
        text(&bad.stderr).trim(),
        "Erro: valor inválido para --laminas: tres."
    );
}

#[test]
fn ei_b11_falta_argumento_e_comando_desconhecido() {
    let base = Base::with_photos(0, (10, 10));
    let missing = base.lamina(&["build", "fotos"]);
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        text(&missing.stderr).trim(),
        "Erro: falta informar --saida."
    );
    let command = base.lamina(&["montar"]);
    assert_eq!(command.status.code(), Some(1));
    assert_eq!(
        text(&command.stderr).trim(),
        "Erro: comando desconhecido: montar."
    );
    let help = base.lamina(&["--help"]);
    assert!(help.status.success());
}
