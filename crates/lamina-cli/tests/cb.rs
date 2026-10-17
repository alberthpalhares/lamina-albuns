//! Cenários da spec `specs/comando-build.md` (CB-B1 a CB-B11), rodando o binário `lamina` de
//! verdade numa pasta temporária.

// Arquivo só de testes (`tests/`): as funções auxiliares também podem usar `unwrap` (AGENTS.md,
// regra 3, vale fora de testes).
#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

mod common;

use std::time::Instant;

use common::{Base, budget, files, text};
use lamina_core::Project;

#[test]
fn cb_b1_album_completo_num_comando() {
    let base = Base::with_photos(12, (300, 200));
    let out = base.build("saida", &["--laminas", "3"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        files(&base.path("saida")),
        [
            "fotos.lamina",
            "lamina-01.jpg",
            "lamina-02.jpg",
            "lamina-03.jpg"
        ]
    );
    let stdout = text(&out.stdout);
    assert!(stdout.contains("lâmina  momento"), "{stdout}");
    assert!(stdout.contains("3 arquivo(s)"), "{stdout}");
}

#[test]
fn cb_b2_sugestao_de_laminas() {
    for (photos, expected) in [
        (40, "Sugestão: 10 lâminas (cerca de 4 fotos por lâmina)."),
        (9, "Sugestão: 2 lâminas"),
        (1, "Sugestão: 1 lâmina ("),
    ] {
        let base = Base::with_photos(photos, (60, 40));
        let out = base.build("saida", &[]);
        assert!(out.status.success(), "{}", text(&out.stderr));
        assert!(
            text(&out.stdout).contains(expected),
            "{photos}: {}",
            text(&out.stdout)
        );
    }
}

#[test]
fn cb_b3_projeto_salvo_com_caminhos_relativos() {
    let base = Base::with_photos(12, (300, 200));
    assert!(base.build("saida", &["--laminas", "3"]).status.success());
    let json = std::fs::read_to_string(base.path("saida/fotos.lamina")).unwrap();
    let project = Project::from_json(&json).unwrap();
    assert_eq!(project.photos.len(), 12);
    for photo in &project.photos {
        let path = photo.path.to_string_lossy().replace('\\', "/");
        assert!(path.starts_with("../fotos/f"), "{path}");
    }
}

#[test]
fn cb_b4_aviso_de_minimo_de_paginas() {
    let base = Base::with_photos(8, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--paginas-minimas", "20"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("O álbum tem 4 páginas; o mínimo pedido é 20."));
    assert_eq!(files(&base.path("saida")).len(), 3, "projeto e 2 JPEGs");
}

#[test]
fn cb_b5_nao_sobrescreve_sem_pedir() {
    let base = Base::with_photos(4, (300, 200));
    std::fs::create_dir_all(base.path("saida")).unwrap();
    std::fs::write(base.path("saida/fotos.lamina"), "antigo").unwrap();
    let out = base.build("saida", &["--laminas", "1"]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr)
            .contains("Já existe fotos.lamina em saida; use --sobrescrever para substituir."),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        files(&base.path("saida")),
        ["fotos.lamina"],
        "nada foi gravado"
    );
    let out = base.build("saida", &["--laminas", "1", "--sobrescrever"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_ne!(
        std::fs::read_to_string(base.path("saida/fotos.lamina")).unwrap(),
        "antigo"
    );
}

#[test]
fn cb_b6_pasta_sem_fotos() {
    let base = Base::with_photos(0, (300, 200));
    let out = base.build("saida", &[]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr).contains("Erro: Nenhuma foto encontrada em fotos."),
        "{}",
        text(&out.stderr)
    );
    let out = base.lamina(&["build", "nao-existe", "--saida", "saida"]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr).contains("Erro: A pasta nao-existe não existe."),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn cb_b7_erros_do_plano_passam_adiante() {
    let base = Base::with_photos(12, (300, 200));
    let out = base.build("saida", &["--laminas", "20"]);
    assert!(!out.status.success());
    assert!(text(&out.stderr).contains("Há 12 fotos para 20 lâminas: use no máximo 12 lâminas."));
}

#[test]
fn cb_b8_especificacao_pela_linha_de_comando() {
    let base = Base::with_photos(12, (300, 200));
    let spec = [
        "--largura-cm",
        "25",
        "--altura-cm",
        "20",
        "--dpi",
        "254",
        "--sangria-mm",
        "2",
    ];
    let out = base.build("saida", &[&["--laminas", "3"][..], &spec[..]].concat());
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        image::image_dimensions(base.path("saida/lamina-01.jpg")).unwrap(),
        (5040, 2040)
    );
    let json = std::fs::read_to_string(base.path("saida/fotos.lamina")).unwrap();
    let project = Project::from_json(&json).unwrap();
    assert!((project.spec.page_width().get() - 250.0).abs() < 1e-9);
    assert_eq!(project.spec.dpi().get(), 254);
}

#[test]
fn cb_b9_reabrir_e_exportar_de_novo() {
    let base = Base::with_photos(12, (300, 200));
    assert!(base.build("saida", &["--laminas", "3"]).status.success());
    let elsewhere = base.path("outra-pasta-de-trabalho");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let project = base.path("saida/fotos.lamina");
    let again = base.path("de-novo");
    let out = base.lamina_in(
        &elsewhere,
        &[
            "exportar",
            project.to_str().unwrap(),
            "--saida",
            again.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    for name in ["lamina-01.jpg", "lamina-02.jpg", "lamina-03.jpg"] {
        let a = std::fs::read(base.path("saida").join(name)).unwrap();
        let b = std::fs::read(again.join(name)).unwrap();
        assert_eq!(a, b, "{name} diferente");
    }
}

#[test]
fn cb_b10_foto_que_sumiu() {
    let base = Base::with_photos(12, (300, 200));
    assert!(base.build("saida", &["--laminas", "3"]).status.success());
    std::fs::remove_file(base.path("fotos/f03.jpg")).unwrap();
    let out = base.lamina(&["exportar", "saida/fotos.lamina", "--saida", "de-novo"]);
    assert!(!out.status.success());
    let stderr = text(&out.stderr);
    assert!(
        stderr.contains("Erro: A foto f03.jpg não foi encontrada em "),
        "{stderr}"
    );
    assert!(
        files(&base.path("de-novo")).is_empty(),
        "nenhum JPEG gravado"
    );
}

#[test]
fn cb_b11_rapido_do_inicio_ao_fim() {
    // Uma base nova a cada medida: com o catálogo já cheio, a importação não seria medida.
    let mut elapsed = std::time::Duration::MAX;
    for _ in 0..lamina_core::testing::HEAVY_TRIES {
        let base = Base::with_photos(40, (3000, 2000));
        let start = Instant::now();
        let out = base.build(
            "saida",
            &["--laminas", "10", "--largura-cm", "30", "--altura-cm", "30"],
        );
        elapsed = elapsed.min(start.elapsed());
        assert!(out.status.success(), "{}", text(&out.stderr));
        assert_eq!(files(&base.path("saida")).len(), 11);
        if elapsed < budget(20_000) {
            break;
        }
    }
    assert!(elapsed < budget(20_000), "{elapsed:?}");
}
