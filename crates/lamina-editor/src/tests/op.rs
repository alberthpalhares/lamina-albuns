//! Cenários da spec `opacidade-da-foto` (OP) que rodam sem janela: a seção da foto, o tamanho do
//! quadro, um arrasto uma edição, a tela, a opacidade que acompanha o quadro e a foto, a IA e a
//! velocidade (OP-B8 a OP-B14).

use lamina_core::testing::{TRIES, budget};
use lamina_core::{Border, FrameContent, Mm, Opacity, PhotoId, Project, RelRect, Rgb};
use serde_json::json;

use super::common::Base;
use super::ia_common::Lab;
use super::tl_common::{H, V, built, editor_at, spec, three_spreads};
use crate::editor::AdjustChange;
use crate::forms::LookField;
use crate::motion::{RowOp, row_ops, strip_rows};
use crate::new_project::Unit;
use crate::{Area, Command, Editor, FrameKind, spread_view};

const CREAM: Rgb = Rgb {
    r: 245,
    g: 240,
    b: 230,
};

/// Os quadros de foto da lâmina atual: (índice, foto).
fn photo_frames(editor: &Editor) -> Vec<(usize, PhotoId)> {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()]
        .frames
        .iter()
        .enumerate()
        .filter_map(|(i, f)| match &f.content {
            FrameContent::Photo { photo, .. } => Some((i, *photo)),
            _ => None,
        })
        .collect()
}

/// A opacidade do quadro `frame` da lâmina atual, em %.
fn opacity(editor: &Editor, frame: usize) -> u8 {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()].frames[frame].opacity.get()
}

/// Muda o quadro `frame` da lâmina atual sem passar pela tela (preparação do cenário).
fn set(editor: &mut Editor, frame: usize, change: impl FnOnce(&mut lamina_core::Frame)) {
    let s = editor.session_mut().unwrap();
    let index = s.current();
    s.edit("preparar", |p| {
        change(&mut p.spreads[index].frames[frame]);
        Ok::<(), crate::EditorError>(())
    })
    .unwrap();
}

fn labels(editor: &Editor) -> Vec<String> {
    editor
        .session()
        .unwrap()
        .history_list()
        .steps
        .iter()
        .map(|st| st.label.clone())
        .collect()
}

#[test]
fn op_b8_a_secao_da_foto() {
    let mut editor = editor_at(three_spreads(), 1);
    let frames = photo_frames(&editor);
    let first = frames[0].0;
    set(&mut editor, first, |f| {
        f.border = Border {
            width: Mm(1.0),
            color: CREAM,
        };
        f.radius = Mm(3.0);
        f.opacity = Opacity::new(70).unwrap();
    });
    editor.select_frame(Some(first));
    let style = editor.photo_style().unwrap();
    assert_eq!(style.opacity.get(), 70);
    assert_eq!(style.border.width, Mm(1.0));
    assert_eq!(style.border.color, CREAM);
    assert_eq!(style.radius, Mm(3.0));
    assert_eq!(editor.photo_style_note(), "");

    assert!(editor.run(Command::Organize));
    for (k, (f, _)) in frames.iter().take(3).enumerate() {
        editor.organize_click(*f, k > 0, false);
    }
    let style = editor.photo_style().unwrap();
    assert_eq!(style.opacity.get(), 70, "os valores do primeiro clicado");
    assert_eq!(
        editor.photo_style_note(),
        "Vale para os 3 quadros selecionados"
    );
    let before = labels(&editor).len();
    assert!(editor.set_photo_opacity(40));
    editor.end_adjust();
    assert!(editor.step_photo_style(LookField::Border, 1));
    assert!(editor.step_photo_style(LookField::Radius, 1));
    assert_eq!(
        labels(&editor).len(),
        before + 3,
        "cada mudança, uma edição"
    );
    let s = editor.session().unwrap();
    let spread = &s.project().spreads[1];
    for (f, _) in frames.iter().take(3) {
        let frame = &spread.frames[*f];
        assert_eq!(frame.opacity.get(), 40);
        assert_eq!(frame.border.width, Mm(1.5));
        assert_eq!(frame.radius, Mm(4.0));
    }
    let other = &spread.frames[frames[3].0];
    assert_eq!(other.opacity, Opacity::FULL, "as outras fotos não mudam");
    assert_eq!(other.border, Border::default());
}

#[test]
fn op_b9_o_tamanho_do_quadro() {
    let spec = spec(30.0, 30.0).with_bleed(Mm(3.0)).unwrap();
    let mut editor = editor_at(built(spec, &[H, H, V, H], 1), 0);
    let frames = photo_frames(&editor);
    // O corte tem 600 × 300 mm: um quadro de 124 × 83 mm e outro encostado no corte.
    let sized = RelRect::new(0.3, 0.2, 124.0 / 600.0, 83.0 / 300.0).unwrap();
    let edge = RelRect::new(0.0, 0.0, 124.0 / 600.0, 83.0 / 300.0).unwrap();
    set(&mut editor, frames[0].0, |f| f.rect = sized);
    set(&mut editor, frames[1].0, |f| f.rect = edge);
    for (frame, _) in &frames[..2] {
        editor.select_frame(Some(*frame));
        for (unit, text) in [
            (Unit::Cm, "Quadro de 12,4 × 8,3 cm"),
            (Unit::Mm, "Quadro de 124 × 83 mm"),
            (Unit::Inch, "Quadro de 4,88 × 3,27 pol"),
        ] {
            editor.remember_unit(unit);
            assert_eq!(editor.photo_style().unwrap().size, text, "só até o corte");
        }
    }
    assert!(editor.run(Command::Organize));
    editor.organize_click(frames[0].0, false, false);
    editor.organize_click(frames[1].0, true, false);
    assert_eq!(
        editor.photo_style().unwrap().size,
        "",
        "com dois, a linha some"
    );
}

#[test]
fn op_b10_um_arrasto_uma_edicao() {
    let mut editor = editor_at(three_spreads(), 0);
    let frame = photo_frames(&editor)[0].0;
    editor.select_frame(Some(frame));
    let before = labels(&editor).len();
    for step in 1..=20u32 {
        let v = u8::try_from(100 - step * 65 / 20).unwrap();
        assert!(editor.set_photo_opacity(v));
    }
    assert_eq!(opacity(&editor, frame), 35);
    editor.end_adjust();
    for v in [40, 45, 50] {
        assert!(editor.set_photo_opacity(v));
    }
    assert!(
        !editor.set_photo_opacity(50),
        "o mesmo valor não cria edição"
    );
    editor.end_adjust();
    let after = labels(&editor);
    assert_eq!(after.len(), before + 2);
    assert!(after[before].starts_with("Opacidade da foto · lâmina 1"));
    assert!(editor.run(Command::Undo));
    assert_eq!(opacity(&editor, frame), 35);
    assert!(editor.run(Command::Undo));
    assert_eq!(opacity(&editor, frame), 100);
}

#[test]
fn op_b11_a_tela_mostra_a_opacidade() {
    let photos: Vec<(u32, u32)> = (0..240).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut editor = editor_at(built(spec(30.0, 30.0), &photos, 60), 4);
    let frame = photo_frames(&editor)[0].0;
    set(&mut editor, frame, |f| {
        f.border = Border {
            width: Mm(2.0),
            color: Rgb::WHITE,
        };
    });
    let before = strip_rows(editor.session().unwrap());
    editor.select_frame(Some(frame));
    assert!(editor.set_photo_opacity(50));
    let project = editor.session().unwrap().project().clone();
    let area = Area {
        width: 1200.0,
        height: 700.0,
        margin: 10.0,
    };
    let view = spread_view(&project, 4, area, &[]).unwrap();
    let f = &view.frames[frame];
    assert!((f.opacity - 0.5).abs() < 1e-9);
    assert!(f.border > 0.0, "a borda continua inteira");
    assert!(matches!(f.kind, FrameKind::Photo { .. }));
    let after = strip_rows(editor.session().unwrap());
    assert_eq!(
        row_ops(&before, &after),
        vec![RowOp::Set { at: 4, from: 4 }],
        "só a linha da lâmina 5 muda na faixa"
    );
}

#[test]
fn op_b12_a_opacidade_fica_com_o_quadro_e_segue_a_foto() {
    let mut editor = editor_at(three_spreads(), 1);
    let frames = photo_frames(&editor);
    let ((q1, a), (q2, b)) = (frames[0], frames[1]);
    set(&mut editor, q1, |f| f.opacity = Opacity::new(40).unwrap());
    // Trocar as fotos: a opacidade fica no quadro.
    assert!(editor.drop_frame(q1, q2));
    let photo_at =
        |e: &Editor, q: usize| match &e.session().unwrap().project().spreads[1].frames[q].content {
            FrameContent::Photo { photo, .. } => *photo,
            other => panic!("{other:?}"),
        };
    assert_eq!((photo_at(&editor, q1), photo_at(&editor, q2)), (b, a));
    assert_eq!((opacity(&editor, q1), opacity(&editor, q2)), (40, 100));
    // De volta, e o layout novo: cada foto mantém a opacidade que tinha.
    assert!(editor.drop_frame(q1, q2));
    assert!(editor.run(Command::NextLayout));
    let of = |e: &Editor, photo: PhotoId| {
        let s = e.session().unwrap();
        s.project().spreads[1]
            .frames
            .iter()
            .find(|f| matches!(&f.content, FrameContent::Photo { photo: p, .. } if *p == photo))
            .unwrap()
            .opacity
            .get()
    };
    assert_eq!((of(&editor, a), of(&editor, b)), (40, 100));
    // Copiar e colar o quadro de A: a cópia vem com a opacidade.
    let qa = photo_frames(&editor)
        .iter()
        .find(|(_, p)| *p == a)
        .unwrap()
        .0;
    assert!(editor.run(Command::Organize));
    editor.organize_click(qa, false, false);
    assert!(editor.run(Command::CopyFrames));
    assert!(editor.run(Command::PasteFrames));
    let s = editor.session().unwrap();
    let last = s.project().spreads[1].frames.last().unwrap();
    assert_eq!(last.opacity.get(), 40, "a cópia vem com a opacidade");
    assert!(editor.run(Command::Undo));
    assert!(editor.run(Command::Organize));
    // "Aplicar a todas" não leva a opacidade às outras lâminas.
    assert!(editor.style_all());
    let p = editor.session().unwrap().project();
    assert!(
        [0, 2]
            .iter()
            .flat_map(|i| &p.spreads[*i].frames)
            .all(|f| f.opacity == Opacity::FULL)
    );
    // "Restaurar a foto" põe A a 100 numa edição "Restaurar ajustes", sem mexer na borda.
    let qa = photo_frames(&editor)
        .iter()
        .find(|(_, p)| *p == a)
        .unwrap()
        .0;
    set(&mut editor, qa, |f| f.border.width = Mm(1.0));
    editor.select_frame(Some(qa));
    assert!(editor.adjust(AdjustChange::Restore));
    assert_eq!(opacity(&editor, qa), 100);
    assert!(
        labels(&editor)
            .last()
            .unwrap()
            .starts_with("Restaurar ajustes")
    );
    let s = editor.session().unwrap();
    assert_eq!(s.project().spreads[1].frames[qa].border.width, Mm(1.0));
}

#[test]
fn op_b12_foto_nova_entra_com_100() {
    let mut editor = editor_at(three_spreads(), 1);
    let frames = photo_frames(&editor);
    for (q, _) in &frames {
        set(&mut editor, *q, |f| f.opacity = Opacity::new(30).unwrap());
    }
    let spare = editor.session().unwrap().project().photos[0].id;
    let place = crate::editor::DropPlace::Spread;
    assert!(editor.drop_photos(place, &[spare]));
    let s = editor.session().unwrap();
    let new = s.project().spreads[1]
        .frames
        .iter()
        .filter(|f| matches!(&f.content, FrameContent::Photo { photo, .. } if *photo == spare))
        .map(|f| f.opacity.get())
        .collect::<Vec<_>>();
    assert!(new.contains(&100), "a foto nova entra a 100%: {new:?}");
}

/// A primeira lâmina com três fotos ou mais (contada a partir de 1) e os quadros dela.
fn spread_with_photos(p: &Project) -> (usize, Vec<usize>) {
    p.spreads
        .iter()
        .enumerate()
        .find_map(|(i, s)| {
            let photos: Vec<usize> = s
                .frames
                .iter()
                .enumerate()
                .filter(|(_, f)| matches!(f.content, FrameContent::Photo { .. }))
                .map(|(k, _)| k)
                .collect();
            (photos.len() >= 3).then_some((i + 1, photos))
        })
        .unwrap()
}

#[test]
fn op_b13_ia_com_opacidade() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let (lamina, photos) = spread_with_photos(lab.project());
    let added = lab.ok(
        "adicionar_texto",
        json!({ "lamina": lamina, "pagina": "esquerda", "texto": "Festa" }),
    );
    let text = usize::try_from(added.data["quadro"].as_u64().unwrap()).unwrap();
    let ops = |lab: &Lab| -> Vec<u8> {
        lab.project().spreads[lamina - 1]
            .frames
            .iter()
            .map(|f| f.opacity.get())
            .collect()
    };
    let steps = lab.labels().len();
    lab.ok("aparencia", json!({ "lamina": lamina, "opacidade": 50 }));
    assert_eq!(lab.labels().len(), steps + 1);
    assert_eq!(
        lab.labels().last().unwrap(),
        &format!("IA: Aparência · lâmina {lamina}")
    );
    let now = ops(&lab);
    assert!(photos.iter().all(|k| now[*k] == 50), "as fotos a 50%");
    assert_eq!(now[text - 1], 100, "o texto não muda");
    let second = photos[1] + 1;
    lab.ok(
        "aparencia",
        json!({ "lamina": lamina, "quadro": second, "opacidade": 30 }),
    );
    let now = ops(&lab);
    assert_eq!(now[photos[1]], 30);
    assert_eq!((now[photos[0]], now[photos[2]]), (50, 50), "só o quadro 2");

    let empty = photos[2] + 1;
    lab.ok("tirar_foto", json!({ "lamina": lamina, "quadro": empty }));
    let before = lab.project().clone();
    let steps = lab.labels().len();
    let refusals = [
        (
            json!({ "lamina": lamina, "opacidade": 120 }),
            "A opacidade vai de 0 a 100.".to_owned(),
        ),
        (
            json!({ "lamina": lamina, "quadro": text, "opacidade": 20 }),
            format!("O quadro {text} da lâmina {lamina} é de texto. Escolha um quadro de foto."),
        ),
        (
            json!({ "lamina": lamina, "quadro": empty, "opacidade": 20 }),
            format!("O quadro {empty} da lâmina {lamina} está vazio. Escolha um quadro com foto."),
        ),
        (
            json!({ "lamina": lamina, "quadro": second, "todas": true, "opacidade": 20 }),
            "Use quadro ou todas, não os dois.".to_owned(),
        ),
    ];
    for (args, message) in refusals {
        let e = lab.call("aparencia", args).unwrap_err();
        assert_eq!(e.to_string(), message);
    }
    assert_eq!(lab.project(), &before, "nada muda");
    assert_eq!(lab.labels().len(), steps, "nenhum passo");
}

#[test]
fn op_b14_arrastar_a_opacidade_rapido() {
    // Uma lâmina com 12 quadros de foto.
    let mut project = lamina_core::Project::new("teste", spec(30.0, 30.0));
    let spread = project.add_spread();
    for i in 0..12u32 {
        let photo = project.add_photo(format!("f{i:02}.jpg"), 3000, 2000);
        let (col, row) = (f64::from(i % 4), f64::from(i / 4));
        let rect = RelRect::new(0.02 + col * 0.245, 0.03 + row * 0.32, 0.22, 0.3).unwrap();
        let content = FrameContent::Photo {
            photo,
            crop: lamina_core::Crop::default(),
        };
        project.add_frame(spread, rect, content).unwrap();
    }
    let mut editor = editor_at(project, 0);
    let frames = photo_frames(&editor);
    assert_eq!(frames.len(), 12);
    editor.select_frame(Some(frames[0].0));
    let area = Area {
        width: 1600.0,
        height: 900.0,
        margin: 10.0,
    };
    let mut best = std::time::Duration::MAX;
    for _ in 0..TRIES {
        let mut worst = std::time::Duration::ZERO;
        for step in 0..20u8 {
            let started = std::time::Instant::now();
            editor.set_photo_opacity(step * 5);
            let s = editor.session().unwrap();
            let view = spread_view(s.project(), 0, area, &[]);
            assert!(view.is_some());
            worst = worst.max(started.elapsed());
        }
        best = best.min(worst);
        if best < budget(16) {
            break;
        }
    }
    assert!(best < budget(16), "um passo do arrasto levou {best:?}");
}
