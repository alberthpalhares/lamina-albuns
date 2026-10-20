//! Cenários da spec `novo-projeto` (NP) da etapa 1 e das etapas: formatos, tamanho escolhido,
//! unidades, margens por borda, resumo em pixels, erro de zona segura, voltar sem perder e os
//! presets com margens por borda. A criação mora em `np_create.rs`; o gerador, no
//! `lamina-layout`; a migração, no `lamina-core`.

use lamina_core::{CropLevel, Margins, Mm, Mounting};

use crate::album::SpecFields;
use crate::new_project::{
    Format, NOT_A_NUMBER, NewProject, SAFE_TOO_BIG, SizeField, SizeForm, Unit, formats,
};
use crate::{Preset, PresetsStore, builtin_presets};

fn preset(name: &str, width_cm: f64, height_cm: f64) -> Preset {
    Preset {
        name: name.to_owned(),
        fields: SpecFields {
            width_cm,
            height_cm,
            ..SpecFields::default()
        },
        limits: lamina_core::PageLimits::default(),
        export: lamina_core::ExportPrefs::default(),
    }
}

#[test]
fn np_b1_presets_separados_por_formato() {
    let mut all = builtin_presets();
    all.push(preset("Meu 35 × 25", 35.0, 25.0));
    let sizes: Vec<(Format, Vec<(f64, f64)>)> = formats(&all)
        .into_iter()
        .map(|(f, list)| {
            let s = list
                .iter()
                .map(|p| (p.fields.width_cm, p.fields.height_cm))
                .collect();
            (f, s)
        })
        .collect();
    assert_eq!(
        sizes,
        [
            (
                Format::Square,
                vec![(20.0, 20.0), (25.0, 25.0), (30.0, 30.0)]
            ),
            (
                Format::Landscape,
                vec![(30.0, 20.0), (40.0, 30.0), (35.0, 25.0)]
            ),
            (Format::Portrait, vec![(20.0, 30.0)]),
        ]
    );
    assert_eq!(Format::Landscape.label(), "Horizontal");
}

#[test]
fn np_b2_tamanho_escolhido_preenche_o_personalizado() {
    let square = builtin_presets()
        .into_iter()
        .find(|p| p.name == "Quadrado 30 × 30")
        .unwrap();
    let mut form = SizeForm::default();
    form.choose(&square);
    assert_eq!(form.label(), "Quadrado 30 × 30");
    form.set(SizeField::Dpi, "350").unwrap();
    assert_eq!(
        form.fields,
        SpecFields {
            dpi: 350,
            ..square.fields.clone()
        }
    );
    assert_eq!(form.label(), "Personalizado (a partir de Quadrado 30 × 30)");
    assert_eq!(SizeForm::default().label(), "Personalizado");
}

#[test]
fn np_b3_unidades_so_mudam_a_exibicao() {
    let mut form = SizeForm::default();
    assert_eq!(form.text(SizeField::Width), "30");
    form.set_unit(Unit::Inch);
    assert_eq!(form.text(SizeField::Width), "11,81");
    form.set(SizeField::Height, "12").unwrap();
    let spec = form.spec().unwrap();
    assert!((spec.page_height().get() - 304.8).abs() < 1e-9);
    assert!(
        (spec.page_width().get() - 300.0).abs() < 1e-9,
        "a largura não mudou"
    );
    // Vírgula e ponto valem; texto que não é número fica de fora, com a mensagem da seção 6.
    form.set_unit(Unit::Cm);
    form.set(SizeField::Width, "30.5").unwrap();
    assert_eq!(form.text(SizeField::Width), "30,5");
    assert_eq!(form.set(SizeField::Width, "trinta"), Err(NOT_A_NUMBER));
    assert_eq!(form.text(SizeField::Width), "30,5");
    // A lâmina aberta mostra o dobro da largura da página.
    form.open_spread = true;
    assert_eq!(form.text(SizeField::Width), "61");
    form.set(SizeField::Width, "60").unwrap();
    assert!((form.fields.width_cm - 30.0).abs() < 1e-9);
}

#[test]
fn np_b4_margens_por_borda() {
    let mut form = SizeForm::default();
    let values = [
        (SizeField::BleedOuter, "3"),
        (SizeField::BleedTopBottom, "5"),
        (SizeField::BleedSpine, "0"),
        (SizeField::SafeOuter, "10"),
        (SizeField::SafeTopBottom, "10"),
        (SizeField::SafeSpine, "20"),
    ];
    form.set_unit(Unit::Mm);
    for (field, text) in values {
        form.set(field, text).unwrap();
    }
    let spec = form.spec().unwrap();
    assert_eq!(spec.bleed(), Margins::new(Mm(3.0), Mm(5.0), Mm(0.0)));
    assert_eq!(
        spec.safe_margins(),
        Margins::new(Mm(10.0), Mm(10.0), Mm(20.0))
    );
    // "Especificação do álbum…" abre com as mesmas margens.
    let mut again = SizeForm::new(SpecFields::from_spec(&spec));
    again.set_unit(Unit::Mm);
    for (field, text) in values {
        assert_eq!(again.text(field), text, "{field:?}");
    }
    // "Igual em todas" copia a externa.
    again.same_margins();
    assert_eq!(again.spec().unwrap().bleed(), Margins::uniform(Mm(3.0)));
    assert_eq!(
        again.spec().unwrap().safe_margins(),
        Margins::uniform(Mm(10.0))
    );
}

#[test]
fn np_b5_resumo_em_pixels_igual_a_exportacao() {
    let form = SizeForm::default();
    let spec = form.spec().unwrap();
    let px = lamina_render::print_size(&spec);
    assert_eq!((px.width, px.height), (7157, 3614));
    assert_eq!(
        form.summary().unwrap(),
        "Lâmina aberta 60 × 30 cm · 2 páginas de 30 × 30 cm · arquivo de 7157 × 3614 px a 300 DPI"
    );
}

#[test]
fn np_b6_erro_de_zona_segura() {
    let mut form = SizeForm::default();
    form.set_unit(Unit::Cm);
    form.set(SizeField::Width, "20").unwrap();
    form.set(SizeField::SafeOuter, "10").unwrap();
    form.set(SizeField::SafeSpine, "10,5").unwrap();
    assert_eq!(form.spec().map(|_| ()), Err(SAFE_TOO_BIG));
    assert_eq!(form.error(), Some(SAFE_TOO_BIG));
    assert_eq!(form.summary(), None);
    let mut steps = NewProject::new(form);
    assert!(!steps.advance(), "Próximo desabilitado");
    assert_eq!(steps.step, 1);
}

#[test]
fn np_b15_voltar_nao_perde_o_digitado() {
    let mut steps = NewProject::new(SizeForm::default());
    assert!(steps.advance());
    steps.mounting = Mounting {
        crop: CropLevel::Whole,
        ..Mounting::default()
    };
    steps.back();
    assert_eq!(steps.step, 1);
    steps.size.set(SizeField::Dpi, "350").unwrap();
    assert!(steps.advance());
    assert_eq!(steps.step, 2);
    assert_eq!(steps.mounting.crop, CropLevel::Whole);
    assert_eq!(steps.size.fields.dpi, 350);
    assert!(steps.advance());
    assert_eq!(steps.step, 3);
    assert!(!steps.advance(), "a etapa 3 é a última");
}

#[test]
fn np_b16_presets_com_margens_por_borda() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("presets.json");
    std::fs::write(
        &file,
        r#"{ "versao": 1, "presets": [ { "nome": "Antigo", "especificacao": {
            "largura_cm": 30.0, "altura_cm": 30.0, "dpi": 300,
            "sangria_mm": 3.0, "zona_segura_mm": 10.0 } } ] }"#,
    )
    .unwrap();
    let mut store = PresetsStore::load(&file);
    assert_eq!(store.warning, None);
    let old = store.find("Antigo").unwrap();
    assert_eq!(old.fields.bleed(), Margins::uniform(Mm(3.0)));
    assert_eq!(old.fields.safe(), Margins::uniform(Mm(10.0)));
    let fields = SpecFields {
        spine_mm: Some(20.0),
        bleed_top_mm: Some(5.0),
        ..SpecFields::default()
    };
    let mut new = preset("Novo", 30.0, 30.0);
    new.fields = fields;
    store.save(new).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"versao\": 2"), "{text}");
    assert!(text.contains("\"sangria_cima_baixo_mm\": 5.0"), "{text}");
    let again = PresetsStore::load(&file);
    let saved = again.find("Novo").unwrap();
    assert_eq!(
        saved.fields.safe(),
        Margins::new(Mm(10.0), Mm(10.0), Mm(20.0))
    );
    assert_eq!(
        saved.fields.bleed(),
        Margins::new(Mm(3.0), Mm(5.0), Mm(3.0))
    );
    assert_eq!(
        again.find("Antigo").unwrap().fields.bleed(),
        Margins::uniform(Mm(3.0))
    );
}
