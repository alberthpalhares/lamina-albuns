//! Cenários da spec `escolha-do-tamanho` (ET): o último tamanho marcado, o resumo de uma linha,
//! o filtro por laboratório, salvar com laboratório, a migração da versão 2, os ajustes da gráfica
//! recolhidos, nomes repetidos e o tamanho de um álbum aberto.

use lamina_core::PageLimits;

use crate::album::SpecFields;
use crate::new_project::{
    Format, SizeField, SizeForm, advanced_error, is_advanced, lab_or_all, labs, last_preset,
    matching_preset, short_summary, size_choices, size_labels,
};
use crate::strips::Strips;
use crate::{Preset, PresetsStore, builtin_presets};

fn preset(name: &str, lab: Option<&str>, width_cm: f64, height_cm: f64) -> Preset {
    Preset {
        name: name.to_owned(),
        fields: SpecFields {
            width_cm,
            height_cm,
            ..SpecFields::default()
        },
        limits: PageLimits::default(),
        export: lamina_core::ExportPrefs::default(),
        lab: lab.map(str::to_owned),
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
        last_size: Some(("Horizontal 30 × 20".to_owned(), None)),
        ..Strips::default()
    };
    let json = serde_json::to_string(&strips).unwrap();
    assert!(json.contains("\"ultimo_tamanho\":[\"Horizontal 30 × 20\",null]"));
    let again: Strips = serde_json::from_str(&json).unwrap();
    let chosen = last_preset(&builtin_presets(), again.last_size.as_ref()).unwrap();
    assert_eq!(chosen.name, "Horizontal 30 × 20");
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
    let square = builtin("Quadrado 25 × 25");
    form.choose(&square);
    assert_eq!(
        short_summary(&form, square.limits).unwrap(),
        "25 × 25 cm · 20 a 80 páginas · 300 DPI · sangria 3 mm"
    );
    let lab = builtin("ProAlbuns 20x20");
    form.choose(&lab);
    assert_eq!(
        short_summary(
            &form,
            PageLimits {
                min: Some(10),
                max: None
            }
        )
        .unwrap(),
        "20 × 20 cm · mínimo de 10 páginas · 300 DPI · sangria 1,5 mm em cima e embaixo"
    );
}

#[test]
fn et_b3_filtro_numa_instalacao_nova() {
    assert_eq!(labs(&builtin_presets()), ["ProAlbuns"]);
    let common: Vec<Preset> = builtin_presets()
        .into_iter()
        .filter(|p| p.lab.is_none())
        .collect();
    assert!(labs(&common).is_empty());
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
    // Com "Todos", nada fica de fora; um laboratório que sumiu volta para "Todos".
    let total: usize = size_choices(&all, None).iter().map(|(_, s)| s.len()).sum();
    assert_eq!(total, all.len());
    assert_eq!(lab_or_all(&all, Some("Lab B")), None);
    assert_eq!(lab_or_all(&all, Some(" Lab A ")), Some("Lab A".to_owned()));
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
    assert_eq!(again.user()[1].lab, None);
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
        .map(|p| (p.name.as_str(), p.lab.clone()))
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
        preset("Quadrado 20 × 20", None, 20.0, 20.0),
    ];
    assert_eq!(
        size_labels(Format::Square, &sizes, true),
        ["Flat 30x30 (Lab A)", "Flat 30x30 (Lab B)", "20 × 20"]
    );
    assert_eq!(
        size_labels(Format::Square, &sizes, false),
        ["Flat 30x30", "Flat 30x30", "20 × 20"]
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
    let spec = builtin("Quadrado 25 × 25").fields.to_spec().unwrap();
    let fields = SpecFields::from_spec(&spec);
    let found = matching_preset(&builtin_presets(), &fields).unwrap();
    assert_eq!(found.name, "Quadrado 25 × 25");
    let form = SizeForm::new(fields);
    assert!(
        short_summary(&form, PageLimits::default())
            .unwrap()
            .starts_with("25 × 25 cm")
    );
}
