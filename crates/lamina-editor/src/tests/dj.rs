//! Cenários da spec `design-janela-principal` (DJ): o painel de 340 px, as abas Lâminas e Sequência,
//! a grade de uma linha, a faixa compacta, o medidor, a sombra da dobra e as seções do Detalhes.

use lamina_core::{AlbumEdge, Edges, PageLimits};

use super::tl_common::{editor_at, three_spreads};
use crate::editor::{Details, album_sections};
use crate::page_meter::{page_badge, page_meter};
use crate::photo_grid::{ZOOM_DEFAULT, default_height};
use crate::{Command, Install, Strip, Strips, StripsStore, spread_strip_height};

fn limits(min: Option<u32>, max: Option<u32>) -> PageLimits {
    PageLimits { min, max }
}

fn open(strips: &Strips) -> [bool; 3] {
    [strips.spreads, strips.timeline, strips.photos]
}

#[test]
fn dj_b1_painel_da_direita_com_340_px_por_padrao() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert_eq!(editor.panel_width(1600), 340);
    assert_eq!(editor.set_panel_width(Some(250), 1600), 260);
    assert_eq!(editor.set_panel_width(None, 1600), 340);
}

#[test]
fn dj_b2_medidor_com_minimo_e_maximo() {
    let meter = page_meter(12, limits(Some(10), Some(120))).unwrap();
    assert_eq!(meter.text, "12 páginas · mínimo 10 · máximo 120");
    assert_eq!(meter.short, "12 de 120 páginas");
    assert!((meter.used - 0.1).abs() < 1e-6);
    assert!((meter.tick.unwrap() - 10.0 / 120.0).abs() < 1e-6);
    assert!(meter.inside);
}

#[test]
fn dj_b3_medidor_fora_da_faixa_e_sem_limites() {
    let meter = page_meter(8, limits(Some(10), None)).unwrap();
    assert_eq!(meter.text, "8 páginas · mínimo 10");
    assert_eq!(meter.short, meter.text);
    assert!(
        (meter.used - 0.4).abs() < 1e-6,
        "a barra vai ao dobro do mínimo"
    );
    assert!(!meter.inside);
    assert_eq!(page_meter(8, limits(None, None)), None);
}

#[test]
fn dj_b4_uma_aba_aberta_por_vez() {
    let mut strips = Strips::fresh();
    assert_eq!(open(&strips), [true, false, true]);
    strips.toggle(Strip::Timeline);
    assert_eq!(open(&strips), [false, true, true], "abrir fecha a outra");
    strips.toggle(Strip::Timeline);
    assert_eq!(open(&strips), [false, false, true], "a aba aberta recolhe");
    strips.toggle(Strip::Spreads);
    assert_eq!(open(&strips), [true, false, true]);
    strips.toggle(Strip::Photos);
    assert_eq!(
        open(&strips),
        [true, false, false],
        "Fotografias não mexe nas abas"
    );
}

#[test]
fn dj_b5_atalhos_das_abas() {
    let mut editor = editor_at(three_spreads(), 0);
    let mut store = StripsStore::default();
    store.strips = Strips::fresh();
    editor.set_strips(store);
    assert!(editor.run(Command::ToggleTimeline));
    assert_eq!(open(&editor.strips()), [false, true, true]);
    assert!(editor.run(Command::ToggleSpreads));
    assert_eq!(open(&editor.strips()), [true, false, true]);
}

#[test]
fn dj_b6_arquivo_antigo_com_as_duas_abertas() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(
        &file,
        r#"{"versao":3,"laminas":true,"linha_do_tempo":true,"fotos":true,"painel":false}"#,
    )
    .unwrap();
    let store = StripsStore::load_for(&file, Install::Existing);
    assert_eq!(open(&store.strips), [true, false, true]);
    assert_eq!(store.warning, None);
}

#[test]
fn dj_b7_botao_das_faixas_nunca_abre_as_duas_abas() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(
        &file,
        r#"{"versao":3,"laminas":false,"linha_do_tempo":false,"fotos":false,"painel":false,"faixas_lembradas":[true,true,true]}"#,
    )
    .unwrap();
    let mut strips = StripsStore::load_for(&file, Install::Existing).strips;
    strips.toggle_bottom();
    assert_eq!(open(&strips), [true, false, true]);
}

#[test]
fn dj_b8_instalacao_nova() {
    let dir = tempfile::tempdir().unwrap();
    let store = StripsStore::load_for(&dir.path().join("janela.json"), Install::New);
    assert_eq!(open(&store.strips), [true, false, true]);
    assert!(!store.strips.panel);
}

#[test]
fn dj_b9_grade_de_fotos_com_uma_linha() {
    assert!((default_height(ZOOM_DEFAULT) - 130.0).abs() < 1e-3);
}

#[test]
fn dj_b10_janela_baixa() {
    assert!((spread_strip_height(700.0) - 96.0).abs() < f32::EPSILON);
    assert!((spread_strip_height(900.0) - 128.0).abs() < f32::EPSILON);
}

#[test]
fn dj_b11_quando_a_sombra_da_dobra_aparece() {
    let mut editor = editor_at(three_spreads(), 0);
    assert!(editor.gutter_shadow_shown(), "normal");
    assert!(editor.run(Command::Organize));
    assert!(!editor.gutter_shadow_shown(), "no modo organizar");
    assert!(editor.run(Command::Organize));
    assert!(editor.gutter_shadow_shown(), "de volta");
    assert!(editor.run(Command::ToggleGutterShadow));
    assert!(!editor.gutter_shadow_shown(), "preferência desligada");

    let mut project = three_spreads();
    project.spec = project
        .spec
        .with_edges(AlbumEdge::SinglePage, AlbumEdge::FullSpread);
    let editor = editor_at(project, 0);
    assert!(!editor.gutter_shadow_shown(), "ponta com página simples");
}

#[test]
fn dj_b12_a_preferencia_da_sombra_fica_guardada() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, r#"{"versao":3,"laminas":true}"#).unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert!(editor.strips().gutter_shadow, "sem o campo, ligada");
    assert!(editor.run(Command::ToggleGutterShadow));
    let again = StripsStore::load_for(&file, Install::Existing);
    assert!(!again.strips.gutter_shadow);
}

#[test]
fn dj_b13_a_sombra_nunca_vai_para_a_impressao() {
    let project = three_spreads();
    let saved = serde_json::to_string(&project).unwrap();
    let mut editor = editor_at(project, 0);
    assert!(editor.run(Command::ToggleGutterShadow));
    // A sombra é só preferência da janela: nada dela chega ao projeto, de onde a exportação lê.
    let after = serde_json::to_string(editor.session().unwrap().project()).unwrap();
    assert_eq!(saved, after);
    let source = include_str!("../../../lamina-render/src/screen.rs");
    assert!(
        !source.contains("gutter"),
        "a renderização não conhece a sombra"
    );
}

#[test]
fn dj_b14_secoes_do_detalhes() {
    let mut project = three_spreads();
    project.page_limits = limits(Some(20), Some(80));
    let editor = editor_at(project, 0);
    let Details::Album(rows) = editor.details() else {
        panic!("sem seleção, o álbum");
    };
    let (album, print) = album_sections(rows.clone());
    let titles: Vec<&str> = album.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        titles,
        ["Lâminas", "Faixa de páginas", "Fotos", "Ajustes", "Unidade"]
    );
    let titles: Vec<&str> = print.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(titles, ["Perfil de saída"]);
    assert_eq!(album.len() + print.len(), rows.len(), "nenhuma linha some");
}

#[test]
fn dj_b15_paginas_nas_miniaturas() {
    let edges = Edges::new(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let badges: Vec<&str> = (0..4).map(|i| page_badge(edges, i, 4)).collect();
    assert_eq!(badges, ["1 pág.", "", "", "1 pág."]);
    assert_eq!(page_badge(Edges::default(), 0, 4), "");
}
