//! "Reduzir movimento" (spec MV, regra 14): as durações da seção 4 e a preferência em
//! `janela.json` (MV-B13 e MV-B14).

use super::tl_common::{editor_at, three_spreads};
use crate::motion::Durations;
use crate::{Command, Editor, Menu, StripsStore};

#[test]
fn duracoes_da_secao_4() {
    let d = Durations::new(false);
    assert_eq!(
        (
            d.slide, d.fade, d.navigate, d.scroll, d.land, d.sharp, d.reorder
        ),
        (180, 120, 200, 250, 150, 120, 150)
    );
}

#[test]
fn mv_b13_reduzir_movimento_zera_todas_as_duracoes() {
    let mut editor = editor_at(three_spreads(), 0);
    assert!(!editor.reduce_motion());
    assert!(editor.run(Command::ToggleReduceMotion));
    assert!(editor.reduce_motion());
    assert_eq!(editor.durations(), Durations::new(true));
    let d = editor.durations();
    for ms in [
        d.slide, d.fade, d.navigate, d.scroll, d.land, d.sharp, d.reorder,
    ] {
        assert_eq!(ms, 0);
    }
    // Trocar o layout e navegar continuam funcionando, sem animação.
    assert!(editor.run(Command::NextLayout));
    assert!(editor.run(Command::NextSpread));
    assert_eq!(editor.durations(), Durations::new(true));
}

#[test]
fn mv_b13_reduzir_movimento_fica_no_menu_visualizar_e_vale_sem_album() {
    assert_eq!(Command::ToggleReduceMotion.menu(), Menu::View);
    assert_eq!(Command::ToggleReduceMotion.label(), "Reduzir movimento");
    assert_eq!(Command::ToggleReduceMotion.shortcut(), None);
    let name = Command::ToggleReduceMotion.name();
    assert_eq!(Command::from_name(name), Some(Command::ToggleReduceMotion));
    let mut editor = Editor::new();
    assert!(editor.available(Command::ToggleReduceMotion));
    assert!(editor.run(Command::ToggleReduceMotion));
    assert!(editor.reduce_motion());
}

#[test]
fn mv_b14_a_preferencia_fica_em_janela_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    // Arquivo de antes da MV: sem o campo.
    std::fs::write(
        &file,
        r#"{"laminas": false, "linha_do_tempo": true, "fotos": true}"#,
    )
    .unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load(&file));
    assert!(!editor.reduce_motion());
    assert!(editor.run(Command::ToggleReduceMotion));
    let text = std::fs::read_to_string(&file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["reduzir_movimento"], serde_json::Value::Bool(true));
    // O arquivo antigo abre a faixa Lâminas uma vez (spec FX, regra 11).
    assert_eq!(json["laminas"], serde_json::Value::Bool(true));
    assert_eq!(json["linha_do_tempo"], serde_json::Value::Bool(true));
    // Na próxima vez, vale o que ficou gravado.
    let mut next = Editor::new();
    next.set_strips(StripsStore::load(&file));
    assert!(next.reduce_motion());
    assert!(next.strips().spreads);
}
