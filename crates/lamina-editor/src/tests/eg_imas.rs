//! Os ímãs a mais do modo organizar (spec EG, regras 7 e 8): mesmo tamanho ao redimensionar e
//! espaçamento igual ao mover, com as marcas na tela (EG-B13 e EG-B14).

use lamina_core::{Project, Rect, RelRect};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::magnets::Mark;
use crate::marks::mark_views;
use crate::new_project::Unit;
use crate::snap::Grip;
use crate::{Command, Editor};

const BOTTOM: Grip = Grip::Resize {
    left: false,
    right: false,
    top: false,
    bottom: true,
};

/// 8 pixels da tela, em mm, nos testes.
const TOL: f64 = 2.0;

/// Uma lâmina (a de índice 1) com os quadros nas posições dadas em mm, contadas do canto do corte.
pub(super) fn album_mm(rects: &[(f64, f64, f64, f64)]) -> Project {
    let photos: Vec<(u32, u32)> = (0..16).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut project = built(spec(30.0, 30.0), &photos, 4);
    project.spreads[1].frames.truncate(rects.len().max(1));
    while project.spreads[1].frames.len() < rects.len() {
        crate::hand::add_empty_frame(&mut project, 1, lamina_core::PageSide::Left).unwrap();
    }
    let trim = project.spec.trim_rect();
    let (tw, th) = (trim.width.get(), trim.height.get());
    for (f, (x, y, w, h)) in project.spreads[1].frames.iter_mut().zip(rects) {
        f.rect = RelRect::new(x / tw, y / th, w / tw, h / th).unwrap();
    }
    project
}

/// No modo organizar, na lâmina 1, só com o quadro `frame` selecionado.
fn organizing(project: Project, frame: usize) -> Editor {
    let mut editor = editor_at(project, 1);
    assert!(editor.run(Command::Organize));
    editor.organize_click(frame, false, false);
    editor
}

fn album_gap(editor: &Editor) -> f64 {
    let p = editor.session().unwrap().project();
    p.spread_gap(&p.spreads[1]).get()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn gaps(marks: &[Mark]) -> Vec<f64> {
    marks
        .iter()
        .filter_map(|m| match m {
            Mark::Gap { from, to, .. } => Some(to - from),
            Mark::Size { .. } => None,
        })
        .collect()
}

#[test]
fn eg_b13_mesma_altura() {
    let project = album_mm(&[(40.0, 40.0, 60.0, 90.0), (140.0, 40.0, 60.0, 70.0)]);
    let mut editor = organizing(project, 1);
    let a = editor.frames_mm()[0];
    // A alça de baixo de B leva a altura a 1 mm da de A (uns 3 px na tela).
    assert!(editor.group_grab(1, BOTTOM));
    assert!(editor.group_drag((0.0, 19.0), true, TOL, false));
    let marks = editor.magnet_marks().to_vec();
    assert!(editor.group_release());
    let b = editor.frames_mm()[1];
    assert!(
        close(b.height.get(), a.height.get()),
        "a altura de B encaixa na de A"
    );
    assert!(
        close(b.y.get(), 40.0 + editor_trim_y(&editor)),
        "a borda de cima fica"
    );
    let sizes: Vec<(Rect, f64)> = marks
        .iter()
        .filter_map(|m| match m {
            Mark::Size {
                rect,
                tall: true,
                mm,
            } => Some((*rect, *mm)),
            _ => None,
        })
        .collect();
    assert_eq!(
        sizes.len(),
        2,
        "a marca aparece nos dois quadros: {marks:?}"
    );
    assert!(sizes.iter().all(|(_, mm)| close(*mm, 90.0)));
    assert!(sizes.iter().any(|(r, _)| *r == a) && sizes.iter().any(|(r, _)| *r == b));

    // Com Alt (sem ímã) fica onde soltei, sem marca.
    assert!(editor.run(Command::Undo));
    assert!(editor.group_grab(1, BOTTOM));
    assert!(editor.group_drag((0.0, 19.0), false, TOL, false));
    assert!(editor.magnet_marks().is_empty());
    assert!(editor.group_release());
    assert!(close(editor.frames_mm()[1].height.get(), 89.0));
}

fn editor_trim_y(editor: &Editor) -> f64 {
    editor.session().unwrap().project().spec.trim_rect().y.get()
}

#[test]
fn eg_b14_espacamento_igual() {
    // A e B separados por um vão que não é o espaço do álbum; C bem à direita de B.
    let probe = organizing(album_mm(&[(10.0, 50.0, 40.0, 40.0)]), 0);
    let gap = album_gap(&probe) + 7.0;
    let b_x = 10.0 + 40.0 + gap;
    let c_x = b_x + 40.0 + 30.0;
    let project = album_mm(&[
        (10.0, 50.0, 40.0, 40.0),
        (b_x, 50.0, 40.0, 40.0),
        (c_x, 50.0, 40.0, 40.0),
    ]);
    let mut editor = organizing(project, 2);
    // Aproximo C de B até o vão ficar 0,8 mm maior que o de A–B.
    assert!(editor.group_grab(2, Grip::Move));
    assert!(editor.group_drag((-(30.0 - gap - 0.8), 0.0), true, TOL, false));
    let marks = editor.magnet_marks().to_vec();
    assert!(editor.group_release());
    let f = editor.frames_mm();
    assert!(
        close(f[2].x.get() - f[1].right().get(), gap),
        "o vão B–C fica igual ao A–B"
    );
    let shown = gaps(&marks);
    assert_eq!(shown.len(), 2, "a marca mostra os dois vãos: {marks:?}");
    assert!(shown.iter().all(|g| close(*g, gap)));
}

#[test]
fn eg_b14_entre_dois_vizinhos_centraliza() {
    // B entre A e C, com 10 mm de um lado e 11,4 do outro.
    let project = album_mm(&[
        (10.0, 50.0, 40.0, 40.0),
        (60.0, 50.0, 40.0, 40.0),
        (111.4, 50.0, 40.0, 40.0),
    ]);
    let mut editor = organizing(project, 1);
    assert!(editor.group_grab(1, Grip::Move));
    assert!(editor.group_drag((0.0, 0.0), true, TOL, false));
    let marks = editor.magnet_marks().to_vec();
    assert!(editor.group_release());
    let f = editor.frames_mm();
    let (left, right) = (
        f[1].x.get() - f[0].right().get(),
        f[2].x.get() - f[1].right().get(),
    );
    assert!(
        close(left, right) && close(left, 10.7),
        "o quadro centraliza: {left} e {right}"
    );
    assert_eq!(gaps(&marks).len(), 2, "os dois vãos aparecem: {marks:?}");
}

#[test]
fn eg_b14_vao_igual_tambem_na_coluna() {
    // A em cima de B (vão de 9 mm); C embaixo de B, a 9,6 mm.
    let project = album_mm(&[
        (40.0, 10.0, 40.0, 30.0),
        (40.0, 49.0, 40.0, 30.0),
        (40.0, 120.0, 40.0, 30.0),
    ]);
    let mut editor = organizing(project, 2);
    assert!(editor.group_grab(2, Grip::Move));
    assert!(editor.group_drag((0.0, -(120.0 - 79.0 - 9.6)), true, TOL, false));
    assert!(editor.group_release());
    let f = editor.frames_mm();
    assert!(close(f[2].y.get() - f[1].bottom().get(), 9.0));
}

#[test]
fn eg_b14_fora_do_modo_o_imã_novo_nao_vale() {
    // Regra 1: fora do modo, o gesto de um quadro segue só a TX (as guias de hoje).
    let project = album_mm(&[
        (10.0, 50.0, 40.0, 40.0),
        (60.0, 50.0, 40.0, 40.0),
        (111.4, 50.0, 40.0, 40.0),
    ]);
    let mut editor = editor_at(project, 1);
    assert!(editor.frame_grab(1, Grip::Move));
    assert!(editor.frame_drag(0.0, 0.0, true, 0.1));
    assert!(editor.magnet_marks().is_empty());
}

#[test]
fn eg_b13_marca_na_unidade_da_tela() {
    let project = album_mm(&[(40.0, 40.0, 60.0, 90.0), (140.0, 40.0, 60.0, 70.0)]);
    let mut editor = organizing(project, 1);
    assert!(editor.group_grab(1, BOTTOM));
    assert!(editor.group_drag((0.0, 19.0), true, TOL, false));
    let project = editor.view_project().unwrap();
    let views = mark_views(&project, editor.magnet_marks(), Unit::Cm);
    assert_eq!(views.len(), 2);
    for v in &views {
        assert_eq!(v.text, "= 9 cm");
        assert!(
            v.line.width == 0.0 && v.line.height > 0.0,
            "a linha da altura é vertical"
        );
    }
    let views = mark_views(&project, editor.magnet_marks(), Unit::Mm);
    assert_eq!(views[0].text, "= 90 mm");
}
