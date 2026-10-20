//! Só o que mudou é atualizado (spec MV, regra 3): a faixa, o navegador, a linha do tempo e o
//! painel de layouts (MV-B15 a MV-B17).

use lamina_core::PhotoId;

use super::tl_common::{H, V, built, editor_at, spec};
use crate::motion::{RowOp, panel_key, photo_rows, row_ops, strip_rows, timeline_rows};
use crate::{Command, Editor, Filter, Part, photo_list};

/// Aplica as operações a `old` e devolve a lista resultante (como a janela faz no modelo).
fn apply<T: Clone>(old: &[T], new: &[T], ops: &[RowOp]) -> Vec<T> {
    let mut out = old.to_vec();
    for op in ops {
        match *op {
            RowOp::Set { at, from } => out[at] = new[from].clone(),
            RowOp::Insert { at, from } => out.insert(at, new[from].clone()),
            RowOp::Remove { at } => {
                out.remove(at);
            }
        }
    }
    out
}

fn pairs(keys: &str) -> Vec<(char, u8)> {
    keys.chars()
        .map(|c| (c.to_ascii_lowercase(), u8::from(c.is_ascii_uppercase())))
        .collect()
}

#[test]
fn listas_iguais_nao_geram_operacao() {
    let a = pairs("abcde");
    assert!(row_ops(&a, &a).is_empty());
}

#[test]
fn valor_mudado_vira_uma_troca_de_linha() {
    let (old, new) = (pairs("abcde"), pairs("abCde"));
    assert_eq!(row_ops(&old, &new), vec![RowOp::Set { at: 2, from: 2 }]);
}

#[test]
fn operacoes_reconstroem_a_lista_nova() {
    let cases = [
        ("abcde", "aebcd"),
        ("abcde", "bcdea"),
        ("abcde", "abde"),
        ("abcde", "abxcde"),
        ("", "abc"),
        ("abc", ""),
        ("abcdef", "fedcba"),
        ("abcde", "aBcxe"),
    ];
    for (old, new) in cases {
        let (old, new) = (pairs(old), pairs(new));
        let ops = row_ops(&old, &new);
        assert_eq!(apply(&old, &new, &ops), new, "{old:?} → {new:?}: {ops:?}");
    }
}

#[test]
fn mover_um_item_e_uma_remocao_e_uma_insercao() {
    let (old, new) = (pairs("abcdefgh"), pairs("acdebfgh"));
    let ops = row_ops(&old, &new);
    let removes = ops
        .iter()
        .filter(|o| matches!(o, RowOp::Remove { .. }))
        .count();
    let inserts = ops
        .iter()
        .filter(|o| matches!(o, RowOp::Insert { .. }))
        .count();
    assert_eq!((removes, inserts, ops.len()), (1, 1, 2), "{ops:?}");
}

fn album(spreads: usize) -> lamina_core::Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

fn strip(editor: &Editor) -> Vec<(lamina_core::SpreadId, crate::motion::StripRow)> {
    strip_rows(editor.session().unwrap())
}

#[test]
fn mv_b15_a_faixa_so_atualiza_o_que_mudou() {
    let mut editor = editor_at(album(60), 4);
    let before = strip(&editor);
    // Trocar o layout da lâmina 5 atualiza só a linha 5.
    assert!(editor.run(Command::NextLayout));
    let after = strip(&editor);
    assert_eq!(
        row_ops(&before, &after),
        vec![RowOp::Set { at: 4, from: 4 }]
    );
    // Mover uma lâmina: uma remoção e uma inserção.
    assert!(editor.move_current_to(9));
    let moved = strip(&editor);
    let ops = row_ops(&after, &moved);
    assert_eq!(ops.len(), 2, "{ops:?}");
    assert_eq!(apply(&after, &moved, &ops), moved);
    // Navegar não mexe na faixa.
    assert!(editor.run(Command::NextSpread));
    assert!(row_ops(&moved, &strip(&editor)).is_empty());
}

fn photos(editor: &Editor) -> Vec<(PhotoId, crate::motion::PhotoRow)> {
    let project = editor.session().unwrap().project();
    photo_rows(&photo_list(project, Filter::All), editor.selection())
}

#[test]
fn mv_b16_o_navegador_so_atualiza_o_que_mudou() {
    let mut project = album(4);
    let x = project.add_photo("x.jpg", 3000, 2000);
    let mut editor = editor_at(project, 1);
    assert!(editor.new_photo_frame(None));
    let frame = editor.selected_frame().unwrap();
    let before = photos(&editor);
    let at = before.iter().position(|(id, _)| *id == x).unwrap();
    // Colocar X num quadro vazio muda só a linha de X.
    assert!(editor.drop_photo(frame, x));
    let after = photos(&editor);
    assert_eq!(row_ops(&before, &after), vec![RowOp::Set { at, from: at }]);
    // Clicar numa foto muda só as linhas da seleção.
    let shown: Vec<PhotoId> = after.iter().map(|(id, _)| *id).collect();
    editor.click_photo(&shown, 2, false, false);
    let picked = photos(&editor);
    assert_eq!(
        row_ops(&after, &picked),
        vec![RowOp::Set { at: 2, from: 2 }]
    );
}

#[test]
fn mv_b17_selecionar_um_quadro_nao_refaz_o_painel_nem_a_linha_do_tempo() {
    let mut editor = editor_at(album(4), 1);
    let project = editor.session().unwrap().project().clone();
    let (panel, line) = (editor.panel(Part::Spread), timeline_rows(&project));
    editor.select_frame(Some(0));
    let again = editor.panel(Part::Spread);
    assert_eq!(
        panel_key(&again, editor.session().unwrap()),
        panel_key(&panel, editor.session().unwrap())
    );
    assert_eq!(again.in_use, panel.in_use);
    let project = editor.session().unwrap().project().clone();
    assert!(row_ops(&line, &timeline_rows(&project)).is_empty());
}

#[test]
fn mv_b17_aplicar_uma_opcao_muda_so_a_opcao_em_uso() {
    let mut editor = editor_at(album(4), 1);
    let before = editor.panel(Part::Spread);
    let key = panel_key(&before, editor.session().unwrap());
    let other = (0..before.options.len())
        .find(|i| Some(*i) != before.in_use)
        .unwrap();
    assert!(editor.apply_option(Part::Spread, other));
    let after = editor.panel(Part::Spread);
    // As prévias são as mesmas: o painel reaproveita as que já fez.
    assert_eq!(panel_key(&after, editor.session().unwrap()), key);
    assert_eq!(after.in_use, Some(other));
}

#[test]
fn favoritar_muda_a_chave_das_previas() {
    let mut editor = editor_at(album(4), 1);
    let key = panel_key(&editor.panel(Part::Spread), editor.session().unwrap());
    assert!(editor.run(Command::Favorite));
    assert_ne!(
        panel_key(&editor.panel(Part::Spread), editor.session().unwrap()),
        key
    );
}
