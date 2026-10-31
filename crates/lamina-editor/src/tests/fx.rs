//! Cenários da spec `faixas-de-baixo` (FX) sobre as faixas e o painel: o botão das faixas de
//! baixo, o clique duplo nos cabeçalhos, a faixa Lâminas aberta e a largura do painel.

use super::tl_common::{editor_at, three_spreads};
use crate::motion::{DOUBLE_CLICK_MS, HeaderClicks};
use crate::{Editor, Install, STRIPS_VERSION, Strip, StripsStore};

fn open(editor: &Editor) -> [bool; 3] {
    let s = editor.strips();
    [s.spreads, s.timeline, s.photos]
}

fn json(file: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

#[test]
fn fx_b5_o_botao_das_faixas_esconde_e_lembra() {
    let mut editor = editor_at(three_spreads(), 0);
    let mut strips = editor.strips();
    strips.toggle(Strip::Timeline);
    assert_eq!(
        [strips.spreads, strips.timeline, strips.photos],
        [true, false, true]
    );
    strips.toggle_bottom();
    assert_eq!([strips.spreads, strips.timeline, strips.photos], [false; 3]);
    strips.toggle_bottom();
    assert_eq!(
        [strips.spreads, strips.timeline, strips.photos],
        [true, false, true],
        "voltam as que estavam abertas"
    );
    // Pelo editor, o mesmo, e grava.
    assert!(editor.toggle_bottom());
    assert_eq!(open(&editor), [false; 3]);
}

#[test]
fn fx_b6_botao_das_faixas_sem_nada_lembrado() {
    let mut editor = editor_at(three_spreads(), 0);
    let mut strips = editor.strips();
    strips.toggle_bottom();
    // Depois de esconder pelo botão, mexer numa faixa sozinha esquece o que ele lembrava.
    strips.toggle(Strip::Spreads);
    strips.toggle(Strip::Spreads);
    assert_eq!([strips.spreads, strips.timeline, strips.photos], [false; 3]);
    strips.toggle_bottom();
    assert_eq!(
        [strips.spreads, strips.timeline, strips.photos],
        [true, false, true]
    );
    assert!(editor.toggle_bottom() && editor.toggle_bottom());
    // Sem nada lembrado, as abas nunca abrem juntas (spec DJ, regra 6).
    assert_eq!(open(&editor), [true, false, true]);
}

#[test]
fn fx_b7_clique_duplo_no_cabecalho() {
    let mut clicks = HeaderClicks::default();
    // Clique e outro clique 250 ms depois: o segundo é do clique duplo e não conta.
    assert!(clicks.accept(1, 0));
    assert!(!clicks.accept(1, 250));
    // Clique e outro 600 ms depois: dois cliques simples, recolhe e abre.
    let mut slow = HeaderClicks::default();
    assert!(slow.accept(1, 1000));
    assert!(slow.accept(1, 1000 + DOUBLE_CLICK_MS + 200));
    // Um clique noutra faixa logo depois conta.
    assert!(slow.accept(2, 1700));
}

#[test]
fn fx_b8_laminas_aberta_para_quem_a_tinha_desligada() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(
        &file,
        r#"{"versao": 2, "laminas": false, "linha_do_tempo": true, "fotos": true, "painel": true}"#,
    )
    .unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert!(editor.strips().spreads, "a faixa Lâminas vem aberta");
    assert_eq!(editor.strips().version, STRIPS_VERSION);
    // Sem arquivo, nas duas instalações, também aberta.
    for install in [Install::New, Install::Existing] {
        let store = StripsStore::load_for(&dir.path().join("nada.json"), install);
        assert!(store.strips.spreads, "{install:?}");
    }
    // Recolhida pelo usuário, fica recolhida na próxima vez.
    assert!(editor.run(crate::Command::ToggleSpreads));
    assert_eq!(json(&file)["versao"], 3);
    let next = StripsStore::load_for(&file, Install::Existing);
    assert!(!next.strips.spreads);
}

#[test]
fn fx_b9_largura_do_painel() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert_eq!(editor.panel_width(1600), 340, "sem nada guardado");
    assert_eq!(editor.set_panel_width(Some(200), 1600), 260);
    assert_eq!(editor.set_panel_width(Some(900), 1600), 720);
    assert_eq!(editor.set_panel_width(None, 1600), 340);
    assert_eq!(json(&file)["painel_largura"], 340);
    // A janela que encolhe só muda o que se mostra.
    assert!(editor.set_panel_width(Some(700), 1600) == 700);
    assert_eq!(editor.panel_width(1000), 450);
    let again = StripsStore::load_for(&file, Install::Existing);
    assert_eq!(again.strips.panel_width, Some(700));
}

#[test]
fn fx_b1_o_vao_pelas_larguras_reais() {
    use crate::motion::{STRIP_SPACING, strip_gap, strip_lefts, strip_shift};
    let widths = [100.0, 50.0, 100.0, 100.0];
    let lefts = strip_lefts(&widths);
    assert_eq!(lefts, [0.0, 110.0, 170.0, 280.0]);
    // O meio da terceira está em 220: passou dele, o vão é depois da terceira.
    assert_eq!(strip_gap(&widths, 0, 219.0), 2);
    assert_eq!(strip_gap(&widths, 0, 230.0), 3);
    // As que correm andam a largura da arrastada (100), não a da estreita (50).
    for i in [1, 2] {
        assert_eq!(strip_shift(&widths, 0, 3, i), -(100.0 + STRIP_SPACING));
    }
    assert_eq!(strip_shift(&widths, 0, 3, 3), 0.0);
    // Levantar a estreita: as outras andam 50.
    assert_eq!(strip_shift(&widths, 1, 4, 3), -(50.0 + STRIP_SPACING));
}

#[test]
fn fx_b2_soltar_move_e_nao_pula() {
    use crate::motion::{strip_drop, strip_landing, strip_lefts, strip_shift};
    let widths = [100.0; 6];
    let (from, gap) = (1, 4);
    // Onde cada miniatura está no último quadro do arrasto.
    let lefts = strip_lefts(&widths);
    let during: Vec<f64> = (0..6)
        .map(|i| {
            if i == from {
                strip_landing(&widths, from, gap)
            } else {
                lefts[i] + strip_shift(&widths, from, gap, i)
            }
        })
        .collect();
    let to = strip_drop(from, gap, 6, true).unwrap();
    assert_eq!(to, 3);
    // Pelo editor: a ordem nova, a lâmina atual e o rótulo.
    let base = super::common::Base::with_photos(12, (600, 400));
    let project = base.project(6);
    let ids: Vec<_> = project.spreads.iter().map(|s| s.id).collect();
    let mut editor = editor_at(project, from);
    assert!(editor.move_current_to(to));
    let s = editor.session().unwrap();
    let now: Vec<_> = s.project().spreads.iter().map(|s| s.id).collect();
    assert_eq!(now, [ids[0], ids[2], ids[3], ids[1], ids[4], ids[5]]);
    assert_eq!(s.current(), 3);
    let list = s.history_list();
    assert_eq!(list.steps.last().unwrap().label, "Mover lâmina · 2 → 4");
    // Depois, cada lâmina fica onde estava no último quadro: sem pulo.
    let after = strip_lefts(&widths);
    for (pos, id) in now.iter().enumerate() {
        let old = ids.iter().position(|x| x == id).unwrap();
        assert!((after[pos] - during[old]).abs() < 1e-9, "lâmina {old}");
    }
}

#[test]
fn fx_b3_desistir() {
    use crate::motion::strip_drop;
    // Soltar fora da faixa (ou Esc, que a janela trata como fora) não move.
    assert_eq!(strip_drop(1, 4, 6, false), None);
    // Soltar no mesmo lugar (o vão antes ou depois dela) também não.
    assert_eq!(strip_drop(1, 1, 6, true), None);
    assert_eq!(strip_drop(1, 2, 6, true), None);
    let mut editor = editor_at(three_spreads(), 1);
    let before = editor.session().unwrap().history_list().steps.len();
    assert!(!editor.move_current_to(1));
    assert_eq!(editor.session().unwrap().history_list().steps.len(), before);
}

#[test]
fn fx_b4_rolar_nas_bordas() {
    use crate::motion::edge_speed;
    let width = 800.0;
    let near = edge_speed(width - 10.0, width);
    let mid = edge_speed(width - 40.0, width);
    assert!(near > mid && mid > 0.0, "{near} {mid}");
    assert_eq!(edge_speed(width - 60.0, width), 0.0);
}

#[test]
fn fx_b10_os_nomes_novos() {
    use crate::{Command, Menu};
    let names: Vec<(&str, &str)> = Command::ALL
        .iter()
        .filter(|c| c.menu() == Menu::View)
        .map(|c| (c.name(), c.label()))
        .collect();
    assert!(names.contains(&("ver.fotos", "Fotografias")));
    assert!(names.contains(&("ver.linha-do-tempo", "Sequência")));
    assert!(names.contains(&("ver.faixas", "Faixas de baixo")));
    assert_eq!(
        Command::from_name("ver.faixas").unwrap().shortcut(),
        Some(crate::Shortcut::alt('0'))
    );
}
