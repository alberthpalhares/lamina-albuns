//! Cenários da spec `fotos-na-sequencia` (SQ): levar uma foto para outra lâmina pela faixa
//! Sequência, mudar a ordem dentro da lâmina, as recusas, onde a foto entra e a velocidade.

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{PhotoId, Project};

use super::tl_common::{H, V, built, editor_at, spec, three_spreads};
use crate::edits::{capacity, layout_with, spread_photos};
use crate::motion::{TIMELINE, TimelineKey, timeline_rows};
use crate::sequence::{SeqSpot, sequence_spot, spot_x};
use crate::{Command, Editor};

fn photos_of(editor: &Editor, spread: usize) -> Vec<PhotoId> {
    spread_photos(editor.session().unwrap().project(), spread)
}

fn last_label(editor: &Editor) -> String {
    let list = editor.session().unwrap().history_list();
    list.steps.last().unwrap().label.clone()
}

#[test]
fn sq_b1_levar_uma_foto_para_outra_lamina_numa_posicao() {
    let mut editor = editor_at(three_spreads(), 0);
    let (a, d) = (photos_of(&editor, 1), photos_of(&editor, 2));
    let b = a[1];
    assert!(editor.sequence_move(1, b, SeqSpot { spread: 2, at: 1 }));
    let mut want_a = a.clone();
    want_a.remove(1);
    let mut want_d = d.clone();
    want_d.insert(1, b);
    assert_eq!(photos_of(&editor, 1), want_a, "a origem fica sem a foto");
    assert_eq!(
        photos_of(&editor, 2),
        want_d,
        "o destino com a foto na posição"
    );
    assert_eq!(last_label(&editor), "Mover foto · lâmina 2 → 3");
    assert_eq!(
        editor.session().unwrap().current(),
        0,
        "a lâmina atual não muda"
    );

    // Pela faixa: a posição da foto na Sequência e o x do ponteiro, como a janela manda.
    let mut editor = editor_at(three_spreads(), 0);
    let rows = timeline_rows(editor.session().unwrap().project());
    let row = rows
        .iter()
        .position(|(k, _)| matches!(k, TimelineKey::Photo(id, _) if *id == b))
        .unwrap();
    let want = SeqSpot { spread: 2, at: 1 };
    let x = (0..4000)
        .map(f64::from)
        .find(|x| editor.sequence_target(*x).map(|t| t.0) == Some(want))
        .unwrap();
    assert!(editor.sequence_drop(row, x));
    assert_eq!(photos_of(&editor, 2), want_d, "o mesmo pela faixa");
    assert!(
        !editor.sequence_drop(rows.len() + 5, x),
        "posição que não existe"
    );
}

#[test]
fn sq_b2_mudar_a_ordem_dentro_da_lamina() {
    let mut editor = editor_at(three_spreads(), 1);
    let a = photos_of(&editor, 1);
    let last = *a.last().unwrap();
    assert!(editor.sequence_move(1, last, SeqSpot { spread: 1, at: 0 }));
    let mut want = a.clone();
    want.pop();
    want.insert(0, last);
    assert_eq!(photos_of(&editor, 1), want);
    assert_eq!(last_label(&editor), "Reordenar fotos · lâmina 2");
    // Soltar no mesmo lugar não muda nada nem entra no histórico.
    let steps = editor.session().unwrap().history_list().steps.len();
    assert!(!editor.sequence_move(1, last, SeqSpot { spread: 1, at: 0 }));
    assert!(!editor.sequence_move(1, last, SeqSpot { spread: 1, at: 1 }));
    assert_eq!(editor.session().unwrap().history_list().steps.len(), steps);
}

#[test]
fn sq_b3_ctrl_z_volta_as_duas_laminas() {
    let mut editor = editor_at(three_spreads(), 0);
    let before = editor.session().unwrap().project().clone();
    let b = photos_of(&editor, 1)[1];
    assert!(editor.sequence_move(1, b, SeqSpot { spread: 2, at: 1 }));
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.session().unwrap().project(), &before);
}

/// Lâmina 1 com uma foto só, lâmina 3 cheia e a lâmina 2 com uma foto da 3 e outras de fora.
fn with_lone_and_full() -> Project {
    let photos: Vec<(u32, u32)> = (0..60).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut p = built(spec(30.0, 30.0), &photos, 10);
    let ids: Vec<PhotoId> = p.photos.iter().map(|f| f.id).collect();
    let full = capacity(&p, 2);
    layout_with(&mut p, 0, &ids[..1]).unwrap();
    layout_with(&mut p, 2, &ids[1..=full]).unwrap();
    layout_with(&mut p, 1, &ids[full - 1..full + 5]).unwrap();
    p
}

#[test]
fn sq_b4_recusas() {
    let mut editor = editor_at(with_lone_and_full(), 1);
    let before = editor.session().unwrap().project().clone();
    let lone = photos_of(&editor, 0)[0];
    assert!(!editor.sequence_move(0, lone, SeqSpot { spread: 1, at: 0 }));
    assert!(editor.take_notice().unwrap().contains("ficaria sem fotos"));
    let middle = photos_of(&editor, 1);
    let shared = middle[0];
    let fresh = middle[2];
    assert!(!editor.sequence_move(1, fresh, SeqSpot { spread: 2, at: 0 }));
    assert!(editor.take_notice().unwrap().contains("o máximo que cabe"));
    assert!(!editor.sequence_move(1, shared, SeqSpot { spread: 2, at: 0 }));
    assert!(
        editor
            .take_notice()
            .unwrap()
            .contains("já está na lâmina 3")
    );
    assert_eq!(
        editor.session().unwrap().project(),
        &before,
        "nada muda nas três"
    );
}

#[test]
fn sq_b5_onde_a_foto_entra() {
    let project = three_spreads();
    let rows = timeline_rows(&project);
    let s = TIMELINE;
    let n0 = spread_photos(&project, 0).len();
    let n2 = spread_photos(&project, 2).len();
    let first = s.padding;
    let spot = |x: f64| sequence_spot(&rows, x, s).unwrap();
    assert_eq!(spot(first + 10.0), SeqSpot { spread: 0, at: 0 });
    assert_eq!(spot(first + s.photo - 10.0), SeqSpot { spread: 0, at: 1 });
    let cut = first + n0 as f64 * (s.photo + s.spacing);
    assert_eq!(
        spot(cut + 2.0),
        SeqSpot { spread: 0, at: n0 },
        "corte, metade da esquerda"
    );
    assert_eq!(
        spot(cut + s.cut - 2.0),
        SeqSpot { spread: 1, at: 0 },
        "metade da direita"
    );
    assert_eq!(
        spot(1.0e6),
        SeqSpot { spread: 2, at: n2 },
        "depois da última"
    );
    // A linha de entrada fica no espaço antes do item seguinte.
    assert!(
        (spot_x(&rows, SeqSpot { spread: 0, at: 0 }, s) - (first - s.spacing / 2.0)).abs() < 1e-9
    );
    assert!(
        (spot_x(&rows, SeqSpot { spread: 0, at: n0 }, s) - (cut - s.spacing / 2.0)).abs() < 1e-9
    );
    let after_cut = cut + s.cut + s.spacing;
    assert!(
        (spot_x(&rows, SeqSpot { spread: 1, at: 0 }, s) - (after_cut - s.spacing / 2.0)).abs()
            < 1e-9
    );
}

#[test]
fn sq_b6_mover_rapido() {
    let photos: Vec<(u32, u32)> = (0..300).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut editor = editor_at(built(spec(30.0, 30.0), &photos, 40), 10);
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let photo = photos_of(&editor, 20)[1];
        let started = Instant::now();
        assert!(editor.sequence_move(20, photo, SeqSpot { spread: 21, at: 1 }));
        best = best.min(started.elapsed());
        assert!(editor.run(Command::Undo));
        if best < budget(10) {
            break;
        }
    }
    assert!(best < budget(10), "mover levou {best:?}");
}
