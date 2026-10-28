//! A opacidade no arquivo (spec OP, regra 2 e seção 6): o formato 6, a migração só de versão e o
//! valor fora da faixa (OP-B1 e OP-B2).

use crate::{
    AlbumSpec, CURRENT_SCHEMA_VERSION, CoreError, Crop, Dpi, FrameContent, Mm, Opacity, Project,
    RelRect,
};

/// Duas lâminas com uma foto cada.
fn project() -> Project {
    let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
    let mut project = Project::new("teste", spec);
    let photo = project.add_photo("fotos/IMG_0001.jpg", 6000, 4000);
    for _ in 0..2 {
        let spread = project.add_spread();
        let content = FrameContent::Photo {
            photo,
            crop: Crop::default(),
        };
        let rect = RelRect::new(0.1, 0.1, 0.4, 0.4).unwrap();
        project.add_frame(spread, rect, content).unwrap();
    }
    project
}

#[test]
fn op_b1_formato_6_abre_o_5_e_volta_igual() {
    // Desde a v7 (specs TM e CD), o arquivo atual faz o papel do 6 aqui.
    const { assert!(CURRENT_SCHEMA_VERSION >= 6) };
    let v6 = project().to_json().unwrap();
    assert!(!v6.contains("opacidade"), "100% não vai para o arquivo");
    let current = format!("\"schema_version\": {CURRENT_SCHEMA_VERSION}");
    let v5 = v6.replace(&current, "\"schema_version\": 5");
    assert_ne!(v5, v6);
    let opened = Project::from_json(&v5).unwrap();
    assert!(
        opened
            .spreads
            .iter()
            .flat_map(|s| &s.frames)
            .all(|f| f.opacity == Opacity::FULL)
    );
    assert_eq!(
        opened.to_json().unwrap(),
        v6,
        "igual ao de entrada a não ser pela versão"
    );
}

#[test]
fn op_b1_arquivo_6_com_opacidade_pede_versao_nova() {
    let mut p = project();
    p.spreads[0].frames[0].opacity = Opacity::new(40).unwrap();
    let json = p.to_json().unwrap();
    assert!(json.contains("\"opacidade\": 40"), "{json}");
    // Desde a v7 (specs TM e CD), o arquivo atual faz o papel do 6 aqui.
    assert!(json.contains(&format!("\"schema_version\": {CURRENT_SCHEMA_VERSION}")));
    assert_eq!(Project::from_json(&json).unwrap(), p, "volta igual");
    // Uma versão que só conhece até a anterior recusa o arquivo em vez de apagar a opacidade: o
    // mesmo caminho de hoje com uma versão mais nova que a do programa.
    let mut newer: serde_json::Value = serde_json::from_str(&json).unwrap();
    newer["schema_version"] = serde_json::json!(CURRENT_SCHEMA_VERSION + 1);
    assert!(matches!(
        Project::from_json(&newer.to_string()),
        Err(CoreError::UnsupportedSchema { found, supported })
            if found == CURRENT_SCHEMA_VERSION + 1 && supported == CURRENT_SCHEMA_VERSION
    ));
}

#[test]
fn op_b2_valor_fora_da_faixa() {
    let json = project().to_json().unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let with = |value: &mut serde_json::Value, v: serde_json::Value| {
        value["spreads"][1]["frames"][0]["opacidade"] = v;
        value.to_string()
    };
    let bad = with(&mut value, serde_json::json!(150));
    assert!(matches!(
        Project::from_json(&bad),
        Err(CoreError::InvalidOpacity {
            value: 150,
            spread: 2
        })
    ));
    for v in [
        serde_json::json!(-1),
        serde_json::json!(50.5),
        serde_json::json!(300),
    ] {
        let text = with(&mut value, v.clone());
        assert!(
            matches!(Project::from_json(&text), Err(CoreError::Json(_))),
            "{v} é JSON inválido"
        );
    }
    assert_eq!(Opacity::new(0).map(Opacity::get), Some(0));
    assert_eq!(Opacity::new(100), Some(Opacity::FULL));
    assert_eq!(Opacity::new(101), None);
}
