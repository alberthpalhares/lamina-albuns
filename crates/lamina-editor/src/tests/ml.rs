//! Fotos entre lâminas no editor (spec ML, regras 1 a 10 e 13): encher, trocar, acrescentar, um
//! Ctrl+Z para as duas lâminas, o alvo pela geometria, o texto que nunca recebe foto, a lâmina
//! cheia, a foto que já está no destino, a mesma lâmina, a origem vazia na tela e não no papel e
//! a velocidade (ML-B1 a B8, B10, B14 a B16).

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{Crop, FrameContent, PhotoId, Project, RelRect, Rgb, TextBox};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::editor::CrossTarget;
use crate::edits::spread_photos;
use crate::motion::{CrossHit, cross_target};
use crate::{Command, DropPlace, Editor, EditorError, Human};

/// Álbum de `spreads` lâminas com 4 fotos cada.
fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

fn project(editor: &Editor) -> &Project {
    editor.session().unwrap().project()
}

fn content(editor: &Editor, spread: usize, frame: usize) -> FrameContent {
    project(editor).spreads[spread].frames[frame]
        .content
        .clone()
}

fn photo(editor: &Editor, spread: usize, frame: usize) -> Option<PhotoId> {
    match content(editor, spread, frame) {
        FrameContent::Photo { photo, .. } => Some(photo),
        _ => None,
    }
}

fn last_label(editor: &Editor) -> String {
    let list = editor.session().unwrap().history_list();
    list.steps
        .last()
        .map(|s| s.label.clone())
        .unwrap_or_default()
}

/// O álbum de ML-B1: a lâmina 5 (índice 4) com o quadro 2 vazio.
fn with_empty_in_five() -> Editor {
    let mut p = album(6);
    p.spreads[4].frames[1].content = FrameContent::Empty;
    editor_at(p, 2)
}

#[test]
fn ml_b1_destino_vazio_enche_e_a_origem_fica_vazia() {
    let mut editor = with_empty_in_five();
    let mut before = project(&editor).clone();
    let a = photo(&editor, 2, 1).unwrap();
    if let FrameContent::Photo { crop, .. } = &mut before.spreads[2].frames[1].content {
        crop.zoom = 1.6;
        crop.center_x = 0.3;
    }
    editor.adopt(before.clone());
    editor.session_mut().unwrap().go_to(2);
    assert!(editor.move_photo(
        1,
        CrossTarget::Frame {
            spread: 4,
            frame: 1
        }
    ));
    let FrameContent::Photo { photo: moved, crop } = content(&editor, 4, 1) else {
        panic!("o quadro vazio devia ter recebido a foto");
    };
    assert_eq!(moved, a);
    assert!((crop.zoom - 1.0).abs() < 1e-9, "zoom em 1");
    assert!((crop.center_x - 0.3).abs() < 1e-9, "o centro de antes");
    assert_eq!(content(&editor, 2, 1), FrameContent::Empty);
    let now = project(&editor);
    assert_eq!(
        now.spreads[2].frames[1].rect,
        before.spreads[2].frames[1].rect
    );
    for (s, f) in [(2, 0), (2, 2), (2, 3), (4, 0), (4, 2), (4, 3)] {
        assert_eq!(now.spreads[s].frames[f], before.spreads[s].frames[f]);
    }
    assert_eq!(last_label(&editor), "Mover foto · lâmina 3 → 5");
    assert_eq!(editor.session().unwrap().current(), 2);
}

#[test]
fn ml_b2_destino_com_foto_troca_as_duas() {
    let mut editor = editor_at(album(6), 2);
    let before = project(&editor).clone();
    let (a, b) = (photo(&editor, 2, 0).unwrap(), photo(&editor, 4, 2).unwrap());
    assert!(editor.move_photo(
        0,
        CrossTarget::Frame {
            spread: 4,
            frame: 2
        }
    ));
    assert_eq!(photo(&editor, 4, 2), Some(a));
    assert_eq!(photo(&editor, 2, 0), Some(b));
    let now = project(&editor);
    assert_eq!(
        now.spreads[2].frames[0].rect,
        before.spreads[2].frames[0].rect
    );
    assert_eq!(
        now.spreads[4].frames[2].rect,
        before.spreads[4].frames[2].rect
    );
    assert_eq!(last_label(&editor), "Trocar fotos · lâminas 3 e 5");
}

#[test]
fn ml_b3_fora_dos_quadros_acrescenta_e_refaz_o_destino() {
    let mut p = album(6);
    let mut text = TextBox::new(Rgb { r: 0, g: 0, b: 0 });
    text.text = "Festa".into();
    let id = p.spreads[4].id;
    let rect = RelRect::new(0.4, 0.85, 0.2, 0.1).unwrap();
    p.add_frame(id, rect, FrameContent::Text(text)).unwrap();
    // A lâmina 5 com 3 fotos e "à mão".
    let keep = spread_photos(&p, 4)[..3].to_vec();
    crate::edits::layout_with(&mut p, 4, &keep).unwrap();
    p.spreads[4].manual = true;
    let mut editor = editor_at(p, 2);
    let before3 = project(&editor).spreads[2].clone();
    let a = photo(&editor, 2, 0).unwrap();
    assert!(editor.move_photo(0, CrossTarget::Spread(4)));
    let now = project(&editor);
    let mut five = spread_photos(now, 4);
    five.sort();
    let mut want = keep.clone();
    want.push(a);
    want.sort();
    assert_eq!(five, want);
    assert!(!now.spreads[4].manual, "a marca à mão sai");
    assert!(
        now.spreads[4]
            .frames
            .iter()
            .any(|f| matches!(f.content, FrameContent::Text(_)))
    );
    let three = &now.spreads[2];
    assert_eq!(three.frames[0].content, FrameContent::Empty);
    assert_eq!(three.frames[1..], before3.frames[1..]);
    assert_eq!(three.manual, before3.manual);
    assert_eq!(last_label(&editor), "Mover foto · lâmina 3 → 5");
}

#[test]
fn ml_b4_um_ctrl_z_volta_as_duas_laminas() {
    let targets = [
        CrossTarget::Frame {
            spread: 4,
            frame: 1,
        },
        CrossTarget::Frame {
            spread: 4,
            frame: 2,
        },
        CrossTarget::Spread(4),
    ];
    for target in targets {
        let mut editor = with_empty_in_five();
        let before = project(&editor).clone();
        assert!(editor.move_photo(0, target));
        let after = project(&editor).clone();
        editor.session_mut().unwrap().go_to(0);
        assert!(editor.run(Command::Undo));
        assert_eq!(project(&editor).spreads, before.spreads, "{target:?}");
        assert_eq!(
            editor.session().unwrap().current(),
            2,
            "vai à primeira das duas"
        );
        assert!(editor.run(Command::Redo));
        assert_eq!(project(&editor).spreads, after.spreads);
    }
}

#[test]
fn ml_b5_alvo_pela_geometria_destaque_soltura_e_pouso() {
    let mut p = album(6);
    let id = p.spreads[4].id;
    p.spreads[4].frames.clear();
    let trim = p.spec.trim_rect();
    let canvas = p.spec.canvas_rect();
    // Um quadro entre 0,1 e 0,4 da largura da lâmina (com a sangria).
    let rel_x = |f: f64| (f * canvas.width.get() - trim.x.get()) / trim.width.get();
    let rect = RelRect::new(rel_x(0.1), 0.2, rel_x(0.4) - rel_x(0.1), 0.5).unwrap();
    let content = FrameContent::Photo {
        photo: p.photos[0].id,
        crop: Crop::default(),
    };
    p.add_frame(id, rect, content).unwrap();
    let mid = 0.5 * canvas.height.get() / canvas.width.get() * 0.9;
    let Some(CrossHit::Frame { frame, rect }) = cross_target(&p, 4, 0.2, mid) else {
        panic!("0,2 devia cair no quadro");
    };
    assert_eq!(frame, 0);
    assert!((rect.x - 0.1).abs() < 1e-6 && (rect.x + rect.width - 0.4).abs() < 1e-6);
    // O mesmo retângulo do destaque de hoje (`drop_target`).
    let (same, r) = crate::motion::drop_target(&p, 4, 0.2, mid).unwrap();
    assert_eq!((same, r), (0, rect));
    assert_eq!(cross_target(&p, 4, 0.45, mid), Some(CrossHit::Spread));
    assert_eq!(cross_target(&p, 4, 1.2, mid), None);
}

#[test]
fn ml_b6_quadro_de_texto_nao_e_alvo() {
    let mut p = album(6);
    let id = p.spreads[4].id;
    // A lâmina 5 com uma foto só, na metade esquerda, e o resto livre.
    p.spreads[4].frames.truncate(1);
    p.spreads[4].frames[0].rect = RelRect::new(0.05, 0.1, 0.4, 0.6).unwrap();
    let photo_rect = p.spreads[4].frames[0].rect;
    let text = |s: &str| {
        let mut t = TextBox::new(Rgb { r: 0, g: 0, b: 0 });
        t.text = s.into();
        FrameContent::Text(t)
    };
    // Um título sozinho embaixo e um texto por cima da primeira foto.
    let title = RelRect::new(0.3, 0.93, 0.4, 0.05).unwrap();
    p.add_frame(id, title, text("Título")).unwrap();
    let over = RelRect::new(
        photo_rect.x(),
        photo_rect.y(),
        photo_rect.width() / 2.0,
        photo_rect.height() / 2.0,
    )
    .unwrap();
    p.add_frame(id, over, text("Legenda")).unwrap();
    let canvas = p.spec.canvas_rect();
    let trim = p.spec.trim_rect();
    let at = |rx: f64, ry: f64| {
        (
            (trim.x.get() + rx * trim.width.get()) / canvas.width.get(),
            (trim.y.get() + ry * trim.height.get()) / canvas.width.get(),
        )
    };
    let (tx, ty) = at(0.5, 0.955);
    assert_eq!(cross_target(&p, 4, tx, ty), Some(CrossHit::Spread));
    let (ox, oy) = at(photo_rect.x() + 0.01, photo_rect.y() + 0.01);
    assert!(matches!(
        cross_target(&p, 4, ox, oy),
        Some(CrossHit::Frame { frame: 0, .. })
    ));
    // Na lâmina atual, a foto do navegador sobre o título entra como foto a mais (escolha 13).
    let mut editor = editor_at(p, 4);
    let free = editor.session_mut().unwrap().project().photos.len();
    let mut q = project(&editor).clone();
    let new = q.add_photo("nova.jpg", 3000, 2000);
    editor.adopt(q);
    editor.session_mut().unwrap().go_to(4);
    let place = match cross_target(project(&editor), 4, tx, ty) {
        Some(CrossHit::Spread) => DropPlace::Spread(4),
        other => panic!("{other:?}"),
    };
    assert!(editor.drop_photos(place, &[new]));
    let now = project(&editor);
    assert!(spread_photos(now, 4).contains(&new));
    assert!(
        now.spreads[4]
            .frames
            .iter()
            .any(|f| matches!(&f.content, FrameContent::Text(t) if t.text == "Título"))
    );
    let _ = free;
}

#[test]
fn ml_b7_lamina_cheia_explica_por_que() {
    let photos: Vec<(u32, u32)> = (0..40).map(|_| H).collect();
    let mut p = built(spec(30.0, 30.0), &photos, 5);
    // A lâmina 5 (índice 4) com 24 fotos: as que estão nela e as das outras, sem repetir.
    let all: Vec<PhotoId> = p.photos.iter().map(|ph| ph.id).collect();
    // 24 fotos, sem as da lâmina 3 (as de 16 a 23), para a foto levada não estar lá.
    let full: Vec<PhotoId> = all[..16].iter().chain(&all[24..32]).copied().collect();
    crate::edits::layout_with(&mut p, 4, &full).unwrap();
    let mut editor = editor_at(p, 2);
    let before = project(&editor).clone();
    let steps = editor.session().unwrap().history_list().steps.len();
    assert!(!editor.move_photo(0, CrossTarget::Spread(4)));
    assert_eq!(project(&editor).spreads, before.spreads);
    assert_eq!(editor.session().unwrap().history_list().steps.len(), steps);
    let err = crate::edits_cross::check_room(&before, 4, &full, &all[33..34]).unwrap_err();
    assert!(matches!(
        err,
        EditorError::SpreadFull {
            spread: 4,
            capacity: 24
        }
    ));
    assert_eq!(
        err.human().title,
        "A lâmina 5 já tem 24 fotos, o máximo que cabe nela."
    );
    assert!(
        editor
            .take_notice()
            .is_some_and(|n| n.contains("A lâmina 5 já tem 24 fotos"))
    );
    // Fotos do navegador no mesmo lugar: nada entra.
    assert!(!editor.drop_photos(DropPlace::Spread(4), &all[33..36]));
    assert_eq!(project(&editor).spreads, before.spreads);
}

#[test]
fn ml_b8_foto_que_ja_esta_no_destino() {
    let mut p = album(6);
    let a = spread_photos(&p, 2)[0];
    // A foto A também na lâmina 5, no quadro 1.
    p.spreads[4].frames[0].content = FrameContent::Photo {
        photo: a,
        crop: Crop::default(),
    };
    let mut editor = editor_at(p, 2);
    let before = project(&editor).clone();
    assert!(!editor.move_photo(0, CrossTarget::Spread(4)));
    assert_eq!(project(&editor).spreads, before.spreads);
    let notice = editor.take_notice().unwrap_or_default();
    assert!(
        notice.contains("Esta foto já está na lâmina 5."),
        "{notice}"
    );
    let err = EditorError::AlreadyInSpread { spread: 4 };
    assert_eq!(err.human().title, "Esta foto já está na lâmina 5.");
    // Sobre o quadro que já tem A: nada, sem aviso e sem passo.
    assert!(!editor.move_photo(
        0,
        CrossTarget::Frame {
            spread: 4,
            frame: 0
        }
    ));
    assert_eq!(editor.take_notice(), None);
    assert!(editor.session().unwrap().history_list().steps.is_empty());
}

#[test]
fn ml_b10_na_mesma_lamina_como_hoje() {
    let mut p = album(4);
    p.spreads[2].frames[3].content = FrameContent::Empty;
    let mut editor = editor_at(p, 2);
    let (a, b) = (photo(&editor, 2, 0).unwrap(), photo(&editor, 2, 1).unwrap());
    assert!(editor.move_photo(
        0,
        CrossTarget::Frame {
            spread: 2,
            frame: 1
        }
    ));
    assert_eq!(
        (photo(&editor, 2, 0), photo(&editor, 2, 1)),
        (Some(b), Some(a))
    );
    assert!(editor.move_photo(
        1,
        CrossTarget::Frame {
            spread: 2,
            frame: 3
        }
    ));
    assert_eq!(photo(&editor, 2, 3), Some(a));
    assert_eq!(content(&editor, 2, 1), FrameContent::Empty);
    let before = project(&editor).clone();
    assert!(!editor.move_photo(0, CrossTarget::Spread(2)));
    assert_eq!(project(&editor).spreads, before.spreads);
}

#[test]
fn ml_b15_fotos_do_navegador_numa_vizinha() {
    let mut p = album(6);
    let x = p.add_photo("x.jpg", 3000, 2000);
    let (y, z) = (
        p.add_photo("y.jpg", 3000, 2000),
        p.add_photo("z.jpg", 2000, 3000),
    );
    // A vizinha 4 (índice 3) com 3 fotos.
    let keep = spread_photos(&p, 3)[..3].to_vec();
    crate::edits::layout_with(&mut p, 3, &keep).unwrap();
    let mut editor = editor_at(p, 2);
    assert!(editor.drop_photos(
        DropPlace::Frame {
            spread: 3,
            frame: 0
        },
        &[x]
    ));
    assert_eq!(photo(&editor, 3, 0), Some(x));
    assert_eq!(last_label(&editor), "Colocar foto · lâmina 4");
    assert!(editor.drop_photos(DropPlace::Spread(3), &[y, z]));
    assert_eq!(spread_photos(project(&editor), 3).len(), 5);
    assert_eq!(last_label(&editor), "Acrescentar 2 fotos · lâmina 4");
    assert_eq!(editor.session().unwrap().current(), 2);
}

#[test]
fn ml_b16_edicoes_entre_laminas_rapido() {
    let photos: Vec<(u32, u32)> = (0..480).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut p = built(spec(30.0, 30.0), &photos, 60);
    p.spreads[31].frames[1].content = FrameContent::Empty;
    let mut editor = editor_at(p, 30);
    let mut best = [Duration::MAX; 3];
    let targets = [
        CrossTarget::Frame {
            spread: 31,
            frame: 1,
        },
        CrossTarget::Frame {
            spread: 31,
            frame: 2,
        },
        CrossTarget::Spread(31),
    ];
    for _ in 0..TRIES {
        for (k, target) in targets.iter().enumerate() {
            let started = Instant::now();
            assert!(editor.move_photo(0, *target), "{target:?}");
            best[k] = best[k].min(started.elapsed());
            assert!(editor.run(Command::Undo));
            editor.session_mut().unwrap().go_to(30);
        }
        if best.iter().all(|b| *b < budget(10)) {
            break;
        }
    }
    for (k, b) in best.iter().enumerate() {
        assert!(*b < budget(10), "a edição {k} levou {b:?}");
    }
}

#[test]
fn ml_b9_pela_faixa_de_laminas_igual() {
    // A fração dentro da miniatura da faixa é a mesma da lâmina: o alvo sai da mesma conta.
    let mut editor = with_empty_in_five();
    let p = project(&editor).clone();
    let view = crate::spread_view(
        &p,
        4,
        crate::Area {
            width: 1.0,
            height: 1.0,
            margin: 0.0,
        },
        &[],
    )
    .unwrap();
    let empty = view.frames[1].rect;
    let canvas = p.spec.canvas_rect();
    let aspect = canvas.height.get() / canvas.width.get();
    // A miniatura: 84 px de altura; o ponto no meio do quadro vazio, em pixels dela.
    let (w, h) = (84.0 / aspect, 84.0);
    let (px, py) = (
        (empty.x + empty.width / 2.0 - view.canvas.x) / view.canvas.width * w,
        (empty.y + empty.height / 2.0 - view.canvas.y) / view.canvas.height * h,
    );
    let (fx, fy) = (px / w, py / w);
    let Some(CrossHit::Frame { frame, .. }) = cross_target(&p, 4, fx, fy) else {
        panic!("o meio do quadro vazio na miniatura devia ser o quadro");
    };
    assert_eq!(frame, 1);
    let a = photo(&editor, 2, 1).unwrap();
    assert!(editor.move_photo(1, CrossTarget::Frame { spread: 4, frame }));
    assert_eq!(photo(&editor, 4, 1), Some(a), "como no ML-B1");
    // Fora dos quadros da miniatura da lâmina 6: acrescenta, como no ML-B3.
    let six = spread_photos(project(&editor), 5).len();
    assert_eq!(
        cross_target(project(&editor), 5, 0.5, 0.01),
        Some(CrossHit::Spread)
    );
    assert!(editor.move_photo(0, CrossTarget::Spread(5)));
    assert_eq!(spread_photos(project(&editor), 5).len(), six + 1);
}

#[test]
fn ml_b11_nada_entre_laminas_no_modo_organizar() {
    let mut editor = editor_at(album(6), 2);
    assert!(editor.run(Command::Organize));
    let before = project(&editor).clone();
    assert!(!editor.move_photo(
        0,
        CrossTarget::Frame {
            spread: 4,
            frame: 1
        }
    ));
    assert!(!editor.move_photo(0, CrossTarget::Spread(4)));
    assert_eq!(project(&editor).spreads, before.spreads);
    // Na janela, o arrasto de foto nem começa no modo organizar (o quadro se move, EG).
    let stage = include_str!("../../../lamina-app/ui/stage.slint");
    assert!(stage.contains("DragUi.far(x, y) && !OrganizeUi.on"));
}
