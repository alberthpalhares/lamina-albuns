//! Alinhar, Igualar, Distribuir e os campos da seleção no modo organizar (spec EG, regra 9 e
//! seção 3): EG-B15 e EG-B16.

use lamina_core::{FrameContent, Rect};

use super::eg_imas::album_mm;
use super::tl_common::editor_at;
use crate::editor::{Field, GROUP_NOT_A_NUMBER, NEED_THREE};
use crate::{Command, Editor};

/// Uma medida do quadro (a borda ou o centro que Alinhar iguala).
type Edge = fn(&Rect) -> f64;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn last_label(editor: &Editor) -> String {
    let s = editor.session().unwrap();
    let label = s.history_list().steps.last().unwrap().label.clone();
    label.trim_end_matches(" · lâmina 2").to_owned()
}

/// No modo organizar, na lâmina 1, com os quadros clicados nesta ordem (o primeiro é a referência).
fn picked(rects: &[(f64, f64, f64, f64)], order: &[usize]) -> Editor {
    let mut editor = editor_at(album_mm(rects), 1);
    assert!(editor.run(Command::Organize));
    for (k, f) in order.iter().enumerate() {
        editor.organize_click(*f, k > 0, false);
    }
    editor
}

fn four() -> Editor {
    picked(
        &[
            (20.0, 40.0, 40.0, 50.0),
            (80.0, 60.0, 50.0, 70.0),
            (150.0, 30.0, 30.0, 40.0),
            (230.0, 50.0, 60.0, 60.0),
        ],
        &[1, 0, 2, 3],
    )
}

fn crops(editor: &Editor) -> Vec<FrameContent> {
    let s = editor.session().unwrap();
    s.project().spreads[1]
        .frames
        .iter()
        .map(|f| f.content.clone())
        .collect()
}

#[test]
fn eg_b15_alinhar_igualar_e_distribuir() {
    let mut editor = four();
    let photos = crops(&editor);
    let reference = editor.frames_mm()[1];

    assert!(editor.run(Command::AlignBottom));
    assert_eq!(last_label(&editor), "Alinhar quadros");
    let f = editor.frames_mm();
    assert!(
        f.iter()
            .all(|r| close(r.bottom().get(), reference.bottom().get()))
    );

    let centers: Vec<f64> = f.iter().map(|r| r.y.get() + r.height.get() / 2.0).collect();
    assert!(editor.run(Command::EqualHeight));
    assert_eq!(last_label(&editor), "Igualar altura");
    let f = editor.frames_mm();
    for (r, c) in f.iter().zip(&centers) {
        assert!(
            close(r.height.get(), 70.0),
            "a altura fica a do primeiro clicado"
        );
        assert!(
            close(r.y.get() + r.height.get() / 2.0, *c),
            "o quadro fica no mesmo centro"
        );
    }
    assert_eq!(
        crops(&editor),
        photos,
        "as fotos mantêm o recorte (o centro)"
    );

    let ends = (f[0], f[3]);
    assert!(editor.run(Command::DistributeAcross));
    assert_eq!(last_label(&editor), "Distribuir quadros");
    let f = editor.frames_mm();
    assert_eq!((f[0], f[3]), ends, "as pontas não mexem");
    let gaps: Vec<f64> = f
        .windows(2)
        .map(|w| w[1].x.get() - w[0].right().get())
        .collect();
    assert!(
        gaps.iter().all(|g| close(*g, gaps[0])),
        "vãos iguais: {gaps:?}"
    );

    // Um Ctrl+Z por comando.
    assert!(editor.run(Command::Undo));
    assert!(editor.run(Command::Undo));
    assert!(editor.run(Command::Undo));
    assert!(close(
        editor.frames_mm()[0].y.get(),
        reference.y.get() - 20.0
    ));
}

#[test]
fn eg_b15_alinhar_nos_seis_lados_usa_o_primeiro_clicado() {
    let cases: [(Command, Edge); 6] = [
        (Command::AlignLeft, |r| r.x.get()),
        (Command::AlignRight, |r| r.right().get()),
        (Command::AlignCenter, |r| r.x.get() + r.width.get() / 2.0),
        (Command::AlignTop, |r| r.y.get()),
        (Command::AlignBottom, |r| r.bottom().get()),
        (Command::AlignMiddle, |r| r.y.get() + r.height.get() / 2.0),
    ];
    for (command, edge) in cases {
        let mut editor = four();
        let reference = editor.frames_mm()[1];
        assert!(editor.run(command), "{command:?}");
        let f = editor.frames_mm();
        assert!(
            f.iter().all(|r| close(edge(r), edge(&reference))),
            "{command:?}"
        );
    }
}

#[test]
fn eg_b15_igualar_tamanho_e_distribuir_na_vertical() {
    let mut editor = picked(
        &[
            (40.0, 10.0, 40.0, 30.0),
            (40.0, 60.0, 60.0, 20.0),
            (40.0, 200.0, 30.0, 40.0),
        ],
        &[0, 1, 2],
    );
    assert!(editor.run(Command::EqualSize));
    assert!(
        editor
            .frames_mm()
            .iter()
            .all(|r| close(r.width.get(), 40.0) && close(r.height.get(), 30.0))
    );
    assert!(editor.run(Command::DistributeDown));
    let f = editor.frames_mm();
    let (a, b) = (
        f[1].y.get() - f[0].bottom().get(),
        f[2].y.get() - f[1].bottom().get(),
    );
    assert!(close(a, b));
}

#[test]
fn eg_b15_distribuir_pede_tres_quadros() {
    let mut editor = picked(
        &[(20.0, 40.0, 40.0, 50.0), (80.0, 60.0, 50.0, 70.0)],
        &[0, 1],
    );
    assert!(!editor.available(Command::DistributeAcross));
    assert_eq!(editor.distribute_hint(), NEED_THREE);
    assert!(editor.available(Command::AlignLeft));
    assert!(!editor.run(Command::DistributeAcross));
    // Fora do modo, nenhum deles vale.
    assert!(editor.run(Command::Organize));
    assert!(!editor.available(Command::AlignLeft));
    assert_eq!(editor.distribute_hint(), "");
}

#[test]
fn eg_b16_campos_da_selecao() {
    let mut editor = picked(
        &[
            (20.0, 40.0, 40.0, 50.0),
            (80.0, 60.0, 50.0, 70.0),
            (150.0, 30.0, 30.0, 40.0),
        ],
        &[0, 1, 2],
    );
    let before = editor.group_mm().unwrap();
    assert!(close(before.x.get(), 20.0) && close(before.width.get(), 160.0));
    // Largura de 16 para 24 cm com "Manter proporção": o grupo amplia por inteiro (1,5×).
    assert_eq!(editor.set_group_field(Field::Width, "24", true), Ok(true));
    assert_eq!(last_label(&editor), "Redimensionar 3 quadros");
    let after = editor.group_mm().unwrap();
    assert!(close(after.width.get(), 240.0));
    assert!(close(after.height.get(), before.height.get() * 1.5));
    assert!(close(after.x.get(), 20.0) && close(after.y.get(), 30.0));
    let f = editor.frames_mm();
    assert!(
        close(f[1].width.get(), 75.0),
        "cada quadro cresce pelo mesmo fator"
    );

    // "abc": a mensagem, e nada muda.
    let steps = editor.session().unwrap().history_list().steps.len();
    assert_eq!(
        editor.set_group_field(Field::Height, "abc", true),
        Err(GROUP_NOT_A_NUMBER)
    );
    assert_eq!(editor.session().unwrap().history_list().steps.len(), steps);
    assert_eq!(editor.group_mm(), Some(after));

    // X move o grupo; os campos aparecem na unidade da tela (cm, a de sempre).
    assert_eq!(editor.set_group_field(Field::X, "5", false), Ok(true));
    assert_eq!(last_label(&editor), "Mover 3 quadros");
    assert_eq!(editor.group_fields()[0], "5");
    assert_eq!(editor.group_fields()[2], "24");
}

#[test]
fn eg_b16_largura_sem_proporcao_estica_so_um_eixo() {
    let mut editor = picked(
        &[(20.0, 40.0, 40.0, 50.0), (80.0, 60.0, 50.0, 70.0)],
        &[0, 1],
    );
    let before = editor.group_mm().unwrap();
    assert_eq!(editor.set_group_field(Field::Width, "22", false), Ok(true));
    let after = editor.group_mm().unwrap();
    assert!(close(after.width.get(), 220.0));
    assert!(close(after.height.get(), before.height.get()));
}
