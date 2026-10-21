//! O grupo do modo organizar pelo mouse (spec EG, regras 4 a 6, 10 e 14): mover, parar no corte,
//! ampliar até sangrar, deformar com Shift, o mínimo e o texto, o ímã do grupo, o zoom e a
//! velocidade (EG-B7 a EG-B12, EG-B22 e EG-B24).

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{FrameContent, Project, Rect, RelRect, TextBox};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::group::{mm_per_px, ruler_origin};
use crate::snap::Grip;
use crate::{Command, Editor};

const CORNER: Grip = Grip::Resize {
    left: false,
    right: true,
    top: false,
    bottom: true,
};

/// Uma lâmina (a de índice 1) com os quadros nas frações do corte dadas.
fn album(rects: &[(f64, f64, f64, f64)]) -> Project {
    let photos: Vec<(u32, u32)> = (0..16).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut project = built(spec(30.0, 30.0), &photos, 4);
    project.spreads[1].frames.truncate(rects.len().max(1));
    // Quadros a mais, vazios e com id próprio.
    while project.spreads[1].frames.len() < rects.len() {
        crate::hand::add_empty_frame(&mut project, 1, lamina_core::PageSide::Left).unwrap();
    }
    for (f, (x, y, w, h)) in project.spreads[1].frames.iter_mut().zip(rects) {
        f.rect = RelRect::new(*x, *y, *w, *h).unwrap();
    }
    project
}

/// No modo organizar, na lâmina 1, com os quadros `frames` selecionados.
fn grouped(project: Project, frames: &[usize]) -> Editor {
    let mut editor = editor_at(project, 1);
    assert!(editor.run(Command::Organize));
    for (k, f) in frames.iter().enumerate() {
        editor.organize_click(*f, k > 0, false);
    }
    assert_eq!(editor.selected_frames(), frames);
    editor
}

fn trim(editor: &Editor) -> Rect {
    editor.session().unwrap().project().spec.trim_rect()
}

fn three() -> Project {
    album(&[
        (0.30, 0.30, 0.10, 0.20),
        (0.45, 0.30, 0.10, 0.20),
        (0.30, 0.55, 0.25, 0.10),
    ])
}

#[test]
fn eg_b7_mover_pelo_corpo() {
    let mut editor = grouped(three(), &[0, 1, 2]);
    let before = editor.frames_mm();
    let photos_before = editor.session().unwrap().project().spreads[1]
        .frames
        .clone();
    assert!(editor.group_grab(1, Grip::Move));
    assert!(editor.group_drag((20.0, 0.0), false, 1.0, false));
    assert!(editor.group_release());
    let after = editor.frames_mm();
    for f in 0..3 {
        assert!((after[f].x.get() - before[f].x.get() - 20.0).abs() < 1e-6);
        assert!((after[f].y.get() - before[f].y.get()).abs() < 1e-6);
    }
    let s = editor.session().unwrap();
    let spread = &s.project().spreads[1];
    for (a, b) in spread.frames.iter().zip(&photos_before) {
        assert_eq!(a.content, b.content, "nenhuma foto troca de quadro");
    }
    assert!(spread.manual);
    let last = s.history_list().steps.last().unwrap().label.clone();
    assert_eq!(last, "Mover 3 quadros · lâmina 2");
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.frames_mm(), before);
}

#[test]
fn eg_b8_grupo_para_no_corte() {
    let mut editor = grouped(three(), &[0, 1, 2]);
    let before = editor.frames_mm();
    assert!(editor.group_grab(0, Grip::Move));
    editor.group_drag((5_000.0, 0.0), false, 1.0, false);
    assert!(editor.group_release());
    let after = editor.frames_mm();
    let right = after.iter().map(|r| r.right().get()).fold(0.0, f64::max);
    assert!(
        (right - trim(&editor).right().get()).abs() < 1e-6,
        "a caixa encosta no corte"
    );
    // As posições relativas não mudam.
    let d = after[0].x.get() - before[0].x.get();
    for f in 0..3 {
        assert!((after[f].x.get() - before[f].x.get() - d).abs() < 1e-6);
        assert!((after[f].width.get() - before[f].width.get()).abs() < 1e-6);
    }
}

#[test]
fn eg_b9_ampliar_ate_sangrar() {
    let mut editor = grouped(three(), &[0, 1, 2]);
    let before = editor.frames_mm();
    let crops: Vec<FrameContent> = editor.session().unwrap().project().spreads[1]
        .frames
        .iter()
        .map(|f| f.content.clone())
        .collect();
    assert!(editor.group_grab(2, CORNER));
    editor.group_drag((5_000.0, 5_000.0), false, 1.0, false);
    assert!(editor.group_release());
    let after = editor.frames_mm();
    let s = after[0].width.get() / before[0].width.get();
    assert!(s > 1.0);
    for f in 0..3 {
        assert!(
            (after[f].width.get() / before[f].width.get() - s).abs() < 1e-6,
            "mesmo fator"
        );
        assert!(
            (after[f].height.get() / before[f].height.get() - s).abs() < 1e-6,
            "proporção"
        );
    }
    let t = trim(&editor);
    let right = after.iter().map(|r| r.right().get()).fold(0.0, f64::max);
    let bottom = after.iter().map(|r| r.bottom().get()).fold(0.0, f64::max);
    assert!(
        (right - t.right().get()).abs() < 1e-6 || (bottom - t.bottom().get()).abs() < 1e-6,
        "o grupo para no corte"
    );
    let now: Vec<FrameContent> = editor.session().unwrap().project().spreads[1]
        .frames
        .iter()
        .map(|f| f.content.clone())
        .collect();
    assert_eq!(now, crops, "os recortes ficam iguais");
}

#[test]
fn eg_b10_shift_deforma() {
    let mut editor = grouped(three(), &[0, 1]);
    let before = editor.frames_mm();
    let crops = editor.session().unwrap().project().spreads[1].frames[0]
        .content
        .clone();
    assert!(editor.group_grab(0, CORNER));
    editor.group_drag((40.0, 5.0), false, 1.0, true);
    assert!(editor.group_release());
    let after = editor.frames_mm();
    let sx = after[0].width.get() / before[0].width.get();
    let sy = after[0].height.get() / before[0].height.get();
    assert!((sx - sy).abs() > 0.05, "fatores diferentes: {sx} e {sy}");
    // A foto mantém o centro e o zoom do recorte (cobre o quadro, sem esticar).
    assert_eq!(
        editor.session().unwrap().project().spreads[1].frames[0].content,
        crops
    );
}

#[test]
fn eg_b11_minimo_e_texto() {
    let mut project = three();
    project.spreads[1].frames[1].content = FrameContent::Text(TextBox {
        size_pt: 24.0,
        ..TextBox::new(lamina_core::Rgb::BLACK)
    });
    let mut editor = grouped(project, &[0, 1]);
    assert!(editor.group_grab(0, CORNER));
    // Reduzir até o limite: o grupo para quando um quadro chega a 5 mm.
    editor.group_drag((-5_000.0, -5_000.0), false, 1.0, false);
    let view = editor.view_project().unwrap().into_owned();
    let small = crate::hand::frame_mm(&view, 1, 0).unwrap();
    let side = small.width.get().min(small.height.get());
    let other = crate::hand::frame_mm(&view, 1, 1).unwrap();
    let side = side.min(other.width.get().min(other.height.get()));
    assert!((side - 5.0).abs() < 1e-6, "menor lado {side}");
    // No mesmo gesto, ampliar 2×: o texto vai a 48 pt (do corpo do começo).
    // O arquivo ainda tem os retângulos do começo do gesto: a largura do grupo dobra a caixa.
    let group_w = {
        let r = editor.frames_mm();
        r[0].right().get().max(r[1].right().get()) - r[0].x.get().min(r[1].x.get())
    };
    editor.group_drag((group_w, 0.0), false, 1.0, false);
    assert!(editor.group_release());
    let s = editor.session().unwrap();
    let FrameContent::Text(t) = &s.project().spreads[1].frames[1].content else {
        panic!("texto")
    };
    assert!((t.size_pt - 48.0).abs() < 1e-6, "{}", t.size_pt);
    assert_eq!(
        crate::group::scaled_text(200.0, 2.0, 2.0),
        300.0,
        "nunca acima de 300"
    );
    assert_eq!(crate::group::scaled_text(24.0, 0.01, 0.01), 4.0);
}

#[test]
fn eg_b12_ima_do_grupo() {
    let editor = grouped(three(), &[0, 1, 2]);
    let project = editor.session().unwrap().project().clone();
    let safe = project.spec.safe_rect(lamina_core::PageSide::Left).x.get();
    // A caixa do grupo a 2 mm da zona segura (a tolerância é 3 mm).
    let left = editor
        .frames_mm()
        .iter()
        .map(|r| r.x.get())
        .fold(f64::MAX, f64::min);
    let dx = safe + 2.0 - left;
    for magnet in [true, false] {
        let mut e = grouped(project.clone(), &[0, 1, 2]);
        assert!(e.group_grab(0, Grip::Move));
        e.group_drag((dx, 0.0), magnet, 3.0, false);
        let lines = e.snap_lines().to_vec();
        assert!(e.group_release());
        let now = e
            .frames_mm()
            .iter()
            .map(|r| r.x.get())
            .fold(f64::MAX, f64::min);
        if magnet {
            assert!(
                (now - safe).abs() < 1e-6,
                "encaixa na zona segura: {now} × {safe}"
            );
            assert!(
                lines
                    .iter()
                    .any(|l| l.vertical && (l.at - safe).abs() < 1e-6)
            );
        } else {
            assert!(
                (now - (safe + 2.0)).abs() < 1e-6,
                "com Alt fica onde soltei"
            );
        }
    }
    // Os quadros do grupo não contam como guia.
    let guides = crate::snap::guides_without(&project, 1, &[0, 1, 2]);
    let own = crate::hand::frame_mm(&project, 1, 1).unwrap().x.get();
    assert!(!guides.xs.iter().any(|x| (x - own).abs() < 1e-9));
}

#[test]
fn eg_b22_reguas_e_zoom() {
    let project = three();
    let fit = mm_per_px(&project, 1_000.0).unwrap();
    let zoomed = mm_per_px(&project, 2_000.0).unwrap();
    assert!(
        (100.0 * zoomed - 100.0 * fit / 2.0).abs() < 1e-9,
        "metade dos mm com 2×"
    );
    let (canvas, trim) = (project.spec.canvas_rect(), project.spec.trim_rect());
    let zero = (trim.x.get() - canvas.x.get(), trim.y.get() - canvas.y.get());
    assert_eq!(ruler_origin(&project), zero, "a régua marca 0 no corte");
    let mut editor = grouped(three(), &[0]);
    // O primeiro + dá 1,5× (spec ZR, regra 4).
    assert!(editor.run(Command::ViewZoomIn));
    assert!((editor.view_zoom() - 1.5).abs() < 1e-9);
    assert!(editor.run(Command::ViewFit));
    assert!((editor.view_zoom() - 1.0).abs() < 1e-9);
}

#[test]
fn eg_b24_grupo_rapido() {
    let rects: Vec<(f64, f64, f64, f64)> = (0..24)
        .map(|i| {
            let (c, r) = (f64::from(i % 6), f64::from(i / 6));
            (0.02 + c * 0.16, 0.02 + r * 0.24, 0.12, 0.2)
        })
        .collect();
    let frames: Vec<usize> = (0..12).collect();
    let mut worst = Duration::MAX;
    for _ in 0..TRIES {
        let mut editor = grouped(album(&rects), &frames);
        assert!(editor.group_grab(0, Grip::Move));
        let mut slowest = Duration::ZERO;
        for step in 1..=30 {
            let started = Instant::now();
            editor.group_drag((f64::from(step), f64::from(step) / 2.0), true, 2.0, false);
            let _ = editor.view_project();
            slowest = slowest.max(started.elapsed());
        }
        assert!(editor.group_release());
        worst = worst.min(slowest);
        if worst < budget(16) {
            break;
        }
    }
    assert!(worst < budget(16), "um passo do grupo levou {worst:?}");
}
