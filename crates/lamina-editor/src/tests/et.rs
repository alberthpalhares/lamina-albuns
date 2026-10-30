//! Cenários da spec `escolha-do-tamanho` (ET): o último tamanho marcado, o resumo de uma linha,
//! o filtro por encadernadora, salvar com encadernadora, a migração da versão 2, os ajustes da
//! gráfica recolhidos, nomes repetidos, o tamanho de um álbum aberto e, nas correções de
//! 2026-10-29, os nomes iguais à medida, a lista de encadernadoras e o último tamanho antigo.

use lamina_core::PageLimits;

use crate::album::SpecFields;
use crate::new_project::{
    ADD_BINDER, ALL_BINDERS, Format, SizeField, SizeForm, advanced_error, binder_list,
    binder_or_all, binders, is_advanced, last_preset, matching_preset, short_summary, size_choices,
    size_labels,
};
use crate::strips::Strips;
use crate::{Preset, PresetsStore, builtin_presets};

fn preset(name: &str, binder: Option<&str>, width_cm: f64, height_cm: f64) -> Preset {
    Preset {
        name: name.to_owned(),
        fields: SpecFields {
            width_cm,
            height_cm,
            ..SpecFields::default()
        },
        limits: PageLimits::default(),
        export: lamina_core::ExportPrefs::default(),
        binder: binder.map(str::to_owned),
        album_type: None,
    }
}

fn builtin(name: &str) -> Preset {
    builtin_presets()
        .into_iter()
        .find(|p| p.name == name)
        .unwrap()
}

#[test]
fn et_b1_o_ultimo_tamanho_vem_escolhido() {
    let strips = Strips {
        last_size: Some(("30x20cm".to_owned(), None)),
        ..Strips::default()
    };
    let json = serde_json::to_string(&strips).unwrap();
    assert!(json.contains("\"ultimo_tamanho\":[\"30x20cm\",null]"));
    let again: Strips = serde_json::from_str(&json).unwrap();
    let chosen = last_preset(&builtin_presets(), again.last_size.as_ref()).unwrap();
    assert_eq!(chosen.name, "30x20cm");
    assert_eq!(Format::of(&chosen.fields), Format::Landscape);
    let mut form = SizeForm::default();
    form.choose(&chosen);
    assert!(
        short_summary(&form, chosen.limits)
            .unwrap()
            .starts_with("30 × 20 cm")
    );
    // Um tamanho que não existe mais não marca nada.
    let gone = ("Apagado".to_owned(), None);
    assert_eq!(last_preset(&builtin_presets(), Some(&gone)), None);
}

#[test]
fn et_b2_o_resumo_acompanha_a_escolha() {
    let mut form = SizeForm::default();
    let square = builtin("25x25cm");
    form.choose(&square);
    assert_eq!(
        short_summary(&form, square.limits).unwrap(),
        "25 × 25 cm · 20 a 80 páginas · 300 DPI · sangria 3 mm"
    );
    let binder = builtin("15x20cm");
    form.choose(&binder);
    assert_eq!(
        short_summary(
            &form,
            PageLimits {
                min: Some(10),
                max: None
            }
        )
        .unwrap(),
        "15 × 20 cm · mínimo de 10 páginas · 300 DPI · sangria 1,5 mm em cima e embaixo"
    );
}

#[test]
fn et_b3_filtro_numa_instalacao_nova() {
    assert_eq!(binders(&builtin_presets()), ["ProAlbuns"]);
    let common: Vec<Preset> = builtin_presets()
        .into_iter()
        .filter(|p| p.binder.is_none())
        .collect();
    assert!(binders(&common).is_empty());
}

#[test]
fn et_b4_filtrar_por_laboratorio() {
    let mut all = builtin_presets();
    all.push(preset("Flat 30x30", Some("Lab A"), 30.0, 30.0));
    let groups = size_choices(&all, Some("Lab A"));
    let counts: Vec<(Format, usize)> = groups.iter().map(|(f, s)| (*f, s.len())).collect();
    assert_eq!(
        counts,
        [
            (Format::Square, 1),
            (Format::Landscape, 0),
            (Format::Portrait, 0)
        ]
    );
    // Sem encadernadora, só os tamanhos padrão (decisão do dono de 2026-10-29); uma encadernadora
    // que sumiu volta para eles.
    let standard: Vec<Preset> = size_choices(&all, None)
        .into_iter()
        .flat_map(|(_, s)| s)
        .collect();
    assert_eq!(standard.len(), 15);
    assert!(standard.iter().all(|p| p.binder.is_none()));
    let proalbuns: usize = size_choices(&all, Some("ProAlbuns"))
        .iter()
        .map(|(_, s)| s.len())
        .sum();
    assert_eq!(proalbuns, 14);
    assert_eq!(binder_or_all(&all, Some("Lab B")), None);
    assert_eq!(
        binder_or_all(&all, Some(" Lab A ")),
        Some("Lab A".to_owned())
    );
}

#[test]
fn et_b5_salvar_com_laboratorio() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("presets.json");
    let mut store = PresetsStore::load(&file);
    store
        .save(preset("Flat 30x30", Some(" Lab A "), 30.0, 30.0))
        .unwrap();
    store
        .save(preset("Só espaços", Some("   "), 20.0, 20.0))
        .unwrap();
    let again = PresetsStore::load(&file);
    let all = again.all();
    let lab_a = size_choices(&all, Some("Lab A"));
    assert_eq!(lab_a[0].1[0].name, "Flat 30x30");
    assert_eq!(again.user()[1].binder, None);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"versao\": 3") && text.contains("\"laboratorio\": \"Lab A\""));
}

#[test]
fn et_b6_migracao_da_versao_2() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("presets.json");
    // Um arquivo da versão 2 é igual ao de hoje sem o campo "laboratorio".
    let presets = [
        preset("Meu 30", None, 30.0, 30.0),
        preset("Meu 20", None, 20.0, 20.0),
    ];
    let list = serde_json::to_string(&presets).unwrap();
    assert!(!list.contains("laboratorio"));
    let old = format!(r#"{{"versao":2,"presets":{list}}}"#);
    std::fs::write(&file, &old).unwrap();
    let store = PresetsStore::load(&file);
    assert_eq!(store.warning, None);
    let names: Vec<(&str, Option<String>)> = store
        .user()
        .iter()
        .map(|p| (p.name.as_str(), p.binder.clone()))
        .collect();
    assert_eq!(names, [("Meu 30", None), ("Meu 20", None)]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), old);
}

#[test]
fn et_b7_ajustes_da_grafica_recolhidos() {
    assert!(!is_advanced(SizeField::Width) && !is_advanced(SizeField::Height));
    assert!(is_advanced(SizeField::Dpi) && is_advanced(SizeField::BleedOuter));
    assert!(is_advanced(SizeField::SafeSpine));
    let mut form = SizeForm::default();
    assert!(!advanced_error(&form, false));
    assert!(
        advanced_error(&form, true),
        "erro na faixa de páginas abre o bloco"
    );
    form.set(SizeField::SafeOuter, "40").unwrap();
    assert!(
        advanced_error(&form, false),
        "zona segura maior que a página abre o bloco"
    );
    let mut zero = SizeForm::default();
    zero.set(SizeField::Width, "0").unwrap();
    assert!(!advanced_error(&zero, false), "a largura fica à vista");
}

#[test]
fn et_b8_nome_repetido_em_laboratorios_diferentes() {
    let sizes = [
        preset("Flat 30x30", Some("Lab A"), 30.0, 30.0),
        preset("Flat 30x30", Some("Lab B"), 30.0, 30.0),
        preset("20x20cm", None, 20.0, 20.0),
    ];
    assert_eq!(
        size_labels(&sizes, true),
        ["Flat 30x30 (Lab A)", "Flat 30x30 (Lab B)", "20x20cm"]
    );
    assert_eq!(
        size_labels(&sizes, false),
        ["Flat 30x30", "Flat 30x30", "20x20cm"]
    );
    let dir = tempfile::tempdir().unwrap();
    let mut store = PresetsStore::load(&dir.path().join("presets.json"));
    store.save(sizes[0].clone()).unwrap();
    store.save(sizes[1].clone()).unwrap();
    assert_eq!(
        store.user().len(),
        2,
        "o mesmo nome em outro laboratório é outro preset"
    );
    assert!(store.exists("Flat 30x30", Some("Lab A")) && !store.exists("Flat 30x30", None));
}

#[test]
fn et_b9_o_mesmo_desenho_na_especificacao_do_album() {
    let spec = builtin("25x25cm").fields.to_spec().unwrap();
    let fields = SpecFields::from_spec(&spec);
    let found = matching_preset(&builtin_presets(), &fields).unwrap();
    assert_eq!(found.name, "25x25cm");
    let form = SizeForm::new(fields);
    assert!(
        short_summary(&form, PageLimits::default())
            .unwrap()
            .starts_with("25 × 25 cm")
    );
}

#[test]
fn et_b10_o_nome_e_a_medida() {
    for p in builtin_presets() {
        let (w, h) = crate::preset_names::parse_size_name(&p.name)
            .unwrap_or_else(|| panic!("{} não é largura x altura em cm", p.name));
        assert_eq!(
            (w, h),
            (p.fields.width_cm, p.fields.height_cm),
            "{}",
            p.name
        );
    }
    let proalbuns: Vec<String> = builtin_presets()
        .into_iter()
        .filter(|p| p.binder.as_deref() == Some("ProAlbuns"))
        .map(|p| p.name)
        .collect();
    assert_eq!(proalbuns.len(), 14, "os preenchidos da tabela da ProAlbuns");
}

#[test]
fn et_b11_encadernadoras_em_ordem_com_busca() {
    let all = [
        preset("A", Some("Zeta"), 30.0, 30.0),
        preset("B", Some("álbuns Beta"), 30.0, 30.0),
        preset("C", Some("ProAlbuns"), 30.0, 30.0),
        preset("D", Some("Zeta"), 20.0, 20.0),
    ];
    assert_eq!(binders(&all), ["álbuns Beta", "ProAlbuns", "Zeta"]);
    assert_eq!(
        binder_list(&all, ""),
        [ALL_BINDERS, "álbuns Beta", "ProAlbuns", "Zeta", ADD_BINDER]
    );
    assert_eq!(binder_list(&all, "beta"), ["álbuns Beta", ADD_BINDER]);
    assert_eq!(
        binder_list(&all, "ALBU"),
        ["álbuns Beta", "ProAlbuns", ADD_BINDER]
    );
    assert_eq!(ALL_BINDERS, "Tamanhos padrão");
    assert_eq!(ADD_BINDER, "Adicionar a minha encadernadora…");
}

#[test]
fn et_b12_o_ultimo_tamanho_com_o_nome_antigo() {
    let all = builtin_presets();
    let old = |name: &str, binder: Option<&str>| {
        let last = (name.to_owned(), binder.map(str::to_owned));
        last_preset(&all, Some(&last)).map(|p| (p.name, p.binder))
    };
    assert_eq!(
        old("Horizontal 30 × 20", None),
        Some(("30x20cm".to_owned(), None))
    );
    assert_eq!(
        old("ProAlbuns 15x20", Some("ProAlbuns")),
        Some(("20x15cm".to_owned(), Some("ProAlbuns".to_owned())))
    );
    assert_eq!(
        old("ProAlbuns 20x20", Some("ProAlbuns")),
        Some(("20x20cm".to_owned(), Some("ProAlbuns".to_owned())))
    );
    assert_eq!(old("30x30cm", None), Some(("30x30cm".to_owned(), None)));
}
