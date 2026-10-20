//! Os ajustes da foto no arquivo (spec AF, regra 9 e seção 6): o formato 5, a migração, as
//! fotos sem ajuste sem campos novos e o projeto com ajuste fora da faixa (AF-B2, AF-B6, AF-B13).

use crate::{AlbumSpec, CURRENT_SCHEMA_VERSION, CoreError, Dpi, Mm, PhotoAdjust, Project};

fn project() -> Project {
    let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
    let mut project = Project::new("teste", spec);
    project.add_photo("fotos/IMG_0001.jpg", 6000, 4000);
    project.add_photo("fotos/IMG_0002.jpg", 4000, 6000);
    project
}

#[test]
fn af_b13_formato_5_abre_o_4_neutro_e_volta_igual() {
    assert_eq!(
        CURRENT_SCHEMA_VERSION, 5,
        "a v4 saiu na beta.4: a AF sobe para a 5"
    );
    let mut v4: serde_json::Value = serde_json::from_str(&project().to_json().unwrap()).unwrap();
    v4["schema_version"] = serde_json::json!(4);
    let opened = Project::from_json(&v4.to_string()).unwrap();
    assert!(opened.photos.iter().all(|p| p.adjust.is_neutral()));
    let saved = opened.to_json().unwrap();
    assert!(saved.contains("\"schema_version\": 5"), "{saved}");

    // Um projeto 5 com ajustes volta igual.
    let mut adjusted = opened.clone();
    adjusted.photos[0].adjust = PhotoAdjust {
        angle_tenths: 3,
        brightness: 20,
        contrast: -10,
        bw: true,
    };
    let json = adjusted.to_json().unwrap();
    assert!(json.contains("\"angulo_decimos\": 3") && json.contains("\"pb\": true"));
    assert_eq!(Project::from_json(&json).unwrap(), adjusted);

    // Uma versão que só conhece a 4 recusa o arquivo novo com o erro de versão (não apaga os
    // ajustes): o mesmo caminho que esta versão usa para a 6.
    let mut newer: serde_json::Value = serde_json::from_str(&json).unwrap();
    newer["schema_version"] = serde_json::json!(CURRENT_SCHEMA_VERSION + 1);
    assert!(matches!(
        Project::from_json(&newer.to_string()),
        Err(CoreError::UnsupportedSchema {
            found: 6,
            supported: 5
        })
    ));
}

#[test]
fn af_b6_fotos_sem_ajuste_nao_ganham_campos() {
    let json = project().to_json().unwrap();
    for field in ["adjust", "angulo_decimos", "brilho", "contraste", "\"pb\""] {
        assert!(
            !json.contains(field),
            "{field} não pode ir para o arquivo: {json}"
        );
    }
}

#[test]
fn af_b2_projeto_com_60_graus_nao_abre() {
    let mut p = project();
    p.photos[1].adjust.angle_tenths = 600;
    let json = p.to_json().unwrap();
    assert!(matches!(
        Project::from_json(&json),
        Err(CoreError::InvalidAdjust { ref photo }) if photo == "IMG_0002.jpg"
    ));
    for bad in [
        PhotoAdjust {
            brightness: 101,
            ..PhotoAdjust::default()
        },
        PhotoAdjust {
            contrast: -101,
            ..PhotoAdjust::default()
        },
        PhotoAdjust {
            angle_tenths: -451,
            ..PhotoAdjust::default()
        },
    ] {
        assert!(!bad.is_valid(), "{bad:?}");
    }
    let edge = PhotoAdjust {
        angle_tenths: -450,
        brightness: 100,
        contrast: -100,
        bw: true,
    };
    assert!(edge.is_valid() && edge.has_tone());
    assert!((edge.degrees() + 45.0).abs() < 1e-12);
}
