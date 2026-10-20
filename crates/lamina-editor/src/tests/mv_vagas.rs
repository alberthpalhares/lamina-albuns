//! As vagas da lâmina grande (spec MV, regras 4 a 8): cada foto mantém o seu lugar na tela entre
//! um estado e outro (MV-B1 a B6, B10 a B12, B25 e B27).

use lamina_core::{FrameContent, PhotoId, Project, Rgb, TextBox};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::motion::{Place, SlotContent, Slots};
use crate::view::{Area, FrameKind, FrameView, PxRect, photo_extent, spread_view};
use crate::{Command, Editor};

fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

/// Os quadros da lâmina `index`, em frações da largura (como a janela recebe).
fn view(project: &Project, index: usize) -> Vec<FrameView> {
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    spread_view(project, index, area, &[]).unwrap().frames
}

fn current(editor: &Editor) -> Vec<FrameView> {
    let s = editor.session().unwrap();
    view(s.project(), s.current())
}

fn place(editor: &Editor) -> Place {
    let s = editor.session().unwrap();
    (1, s.project().spreads[s.current()].id)
}

/// Um quadro com o id de `like` e o conteúdo e o retângulo dados.
fn frame(like: &FrameView, kind: FrameKind, (x, y, w, h): (f64, f64, f64, f64)) -> FrameView {
    let rect = PxRect {
        x,
        y,
        width: w,
        height: h,
    };
    FrameView {
        rect,
        trim: rect,
        kind,
        ..like.clone()
    }
}

fn photo(id: PhotoId) -> FrameKind {
    FrameKind::Photo {
        id,
        source: PxRect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        missing: false,
    }
}

fn text() -> FrameKind {
    FrameKind::Text {
        text: TextBox::new(Rgb::BLACK),
        size_mm: (50.0, 20.0),
    }
}

fn ids(slots: &Slots) -> Vec<u64> {
    slots.slots().iter().map(|s| s.id).collect()
}

fn slot_of(slots: &Slots, content: SlotContent) -> &crate::motion::Slot {
    slots
        .slots()
        .iter()
        .find(|s| s.live && s.content == content)
        .unwrap()
}

#[test]
fn mv_b1_trocar_layout_desliza_cada_foto() {
    let mut editor = editor_at(album(4), 1);
    let mut slots = Slots::default();
    assert!(slots.update(&current(&editor), place(&editor), false));
    let (before, contents): (Vec<u64>, Vec<SlotContent>) =
        slots.slots().iter().map(|s| (s.id, s.content)).unzip();
    assert!(editor.run(Command::NextLayout));
    assert!(!slots.update(&current(&editor), place(&editor), false));
    // Nenhuma vaga nova: as mesmas, com as mesmas fotos, nos retângulos novos.
    assert_eq!(ids(&slots), before);
    let after: Vec<SlotContent> = slots.slots().iter().map(|s| s.content).collect();
    assert_eq!(after, contents);
    assert!(slots.slots().iter().all(|s| s.live && !s.born));
    assert!(slots.slots().iter().any(|s| s.fly));
    for s in slots.slots() {
        let f = &current(&editor)[s.frame.unwrap()];
        assert_eq!(s.rect, f.rect);
    }
}

#[test]
fn mv_b2_foto_que_sai_e_foto_que_entra() {
    let project = album(4);
    let (old, pool) = (view(&project, 1), view(&project, 3));
    let p: Vec<PhotoId> = project.photos.iter().map(|p| p.id).collect();
    let (a, b, c, d) = (p[0], p[1], p[2], p[3]);
    let before = vec![
        frame(&old[0], photo(a), (0.0, 0.0, 0.3, 0.3)),
        frame(&old[1], photo(b), (0.3, 0.0, 0.3, 0.3)),
        frame(&old[2], photo(c), (0.6, 0.0, 0.3, 0.3)),
    ];
    // A lâmina passa a ter A, B e D (quadros novos, com outros ids).
    let after = vec![
        frame(&pool[0], photo(a), (0.0, 0.1, 0.4, 0.3)),
        frame(&pool[1], photo(b), (0.4, 0.1, 0.4, 0.3)),
        frame(&pool[2], photo(d), (0.8, 0.1, 0.2, 0.3)),
    ];
    let mut slots = Slots::default();
    let here = (1, project.spreads[1].id);
    slots.update(&before, here, false);
    let c_slot = slot_of(&slots, SlotContent::Photo(c)).id;
    slots.update(&after, here, false);
    let gone = slots.slots().iter().find(|s| s.id == c_slot).unwrap();
    assert!(!gone.live, "C esmaece");
    assert_eq!(gone.rect, before[2].rect, "no lugar antigo");
    let new = slot_of(&slots, SlotContent::Photo(d));
    assert!(new.born && new.live && !new.fly);
    assert!(slot_of(&slots, SlotContent::Photo(a)).fly);
    assert!(slot_of(&slots, SlotContent::Photo(b)).fly);
    assert_eq!(slots.slots().len(), 4);
}

#[test]
fn mv_b3_trocar_fotos_cruza_as_duas() {
    let mut editor = editor_at(album(4), 1);
    let mut slots = Slots::default();
    let first = current(&editor);
    slots.update(&first, place(&editor), false);
    let (FrameKind::Photo { id: a, .. }, FrameKind::Photo { id: b, .. }) =
        (&first[0].kind, &first[1].kind)
    else {
        panic!("dois quadros de foto")
    };
    let (sa, sb) = (
        slot_of(&slots, SlotContent::Photo(*a)).id,
        slot_of(&slots, SlotContent::Photo(*b)).id,
    );
    assert!(editor.drop_frame(0, 1));
    slots.update(&current(&editor), place(&editor), false);
    let now = current(&editor);
    let slot_a = slot_of(&slots, SlotContent::Photo(*a));
    let slot_b = slot_of(&slots, SlotContent::Photo(*b));
    assert_eq!((slot_a.id, slot_b.id), (sa, sb));
    assert_eq!(slot_a.rect, now[slot_a.frame.unwrap()].rect);
    assert_eq!(slot_a.rect, first[1].rect);
    assert_eq!(slot_b.rect, first[0].rect);
    assert!(slot_a.fly && slot_b.fly);
}

#[test]
fn mv_b4_espelhar_mantem_o_conteudo_e_o_texto_casa_pelo_id() {
    let mut editor = editor_at(album(4), 1);
    assert!(editor.new_text(None));
    let mut slots = Slots::default();
    slots.update(&current(&editor), place(&editor), false);
    let text_slot = slot_of(&slots, SlotContent::Text).clone();
    let before = ids(&slots);
    assert!(editor.run(Command::Mirror));
    slots.update(&current(&editor), place(&editor), false);
    assert_eq!(ids(&slots), before);
    let now = slot_of(&slots, SlotContent::Text);
    assert_eq!(now.id, text_slot.id);
    assert_eq!(now.frame_id, text_slot.frame_id);
    for s in slots.slots() {
        assert!(s.live);
        assert_eq!(s.rect, current(&editor)[s.frame.unwrap()].rect);
    }
}

#[test]
fn mv_b5_texto_pela_ordem_vazio_pelo_mais_proximo_e_texto_maior_nao_desliza() {
    let project = album(4);
    let (old, pool) = (view(&project, 1), view(&project, 3));
    let before = vec![
        frame(&old[0], text(), (0.1, 0.1, 0.3, 0.1)),
        frame(&old[1], FrameKind::Empty, (0.6, 0.1, 0.2, 0.2)),
        frame(&old[2], FrameKind::Empty, (0.1, 0.4, 0.2, 0.2)),
    ];
    // Ids recriados: o texto muda de tamanho; os vazios trocam de ordem e andam um pouco.
    let after = vec![
        frame(&pool[0], FrameKind::Empty, (0.12, 0.42, 0.2, 0.2)),
        frame(&pool[1], text(), (0.1, 0.1, 0.4, 0.1)),
        frame(&pool[2], FrameKind::Empty, (0.62, 0.12, 0.2, 0.2)),
    ];
    let mut slots = Slots::default();
    let here = (1, project.spreads[1].id);
    slots.update(&before, here, false);
    let start: Vec<u64> = ids(&slots);
    slots.update(&after, here, false);
    assert_eq!(ids(&slots), start, "nenhuma vaga nova");
    let s = slots.slots();
    assert_eq!(
        (s[0].frame, s[1].frame, s[2].frame),
        (Some(1), Some(2), Some(0))
    );
    assert!(
        !s[0].fly,
        "o texto mudou de tamanho: aparece no tamanho novo"
    );
    assert!(s[1].fly && s[2].fly);
}

#[test]
fn mv_b6_foto_repetida_fica_pela_ordem() {
    let project = album(4);
    let old = view(&project, 1);
    let a = project.photos[0].id;
    let before = vec![
        frame(&old[0], photo(a), (0.0, 0.0, 0.4, 0.4)),
        frame(&old[1], photo(a), (0.5, 0.0, 0.4, 0.4)),
    ];
    // Espelhar: os dois quadros trocam de lado.
    let after = vec![
        frame(&old[0], photo(a), (0.6, 0.0, 0.4, 0.4)),
        frame(&old[1], photo(a), (0.1, 0.0, 0.4, 0.4)),
    ];
    let mut slots = Slots::default();
    let here = (1, project.spreads[1].id);
    slots.update(&before, here, false);
    slots.update(&after, here, false);
    let s = slots.slots();
    assert_eq!((s[0].frame, s[1].frame), (Some(0), Some(1)));
    assert_eq!((s[0].rect, s[1].rect), (after[0].rect, after[1].rect));
}

#[test]
fn mv_b10_gesto_nao_anima() {
    let project = album(4);
    let old = view(&project, 1);
    let mut moved = old.clone();
    moved[0].rect.x += 0.05;
    let mut slots = Slots::default();
    let here = (1, project.spreads[1].id);
    slots.update(&old, here, false);
    let before = ids(&slots);
    assert!(!slots.update(&moved, here, true));
    assert_eq!(ids(&slots), before);
    assert!(slots.slots().iter().all(|s| !s.fly && s.live && !s.born));
    assert_eq!(slots.slots()[0].rect, moved[0].rect);
}

#[test]
fn mv_b11_outro_album_nao_anima() {
    let project = album(4);
    let frames = view(&project, 1);
    let id = project.spreads[1].id;
    let mut slots = Slots::default();
    slots.update(&frames, (1, id), false);
    // O álbum B tem os mesmos ids de lâmina: o número do álbum é outro.
    let other = view(&project, 2);
    assert!(slots.update(&other, (2, id), false));
    assert!(slots.slots().iter().all(|s| !s.fly && !s.born && s.live));
    assert_eq!(slots.slots().len(), other.len());
}

#[test]
fn mv_b12_vagas_que_sumiram_sao_retiradas() {
    let project = album(4);
    let old: Vec<FrameView> = view(&project, 1)
        .into_iter()
        .chain(view(&project, 2))
        .collect();
    let p: Vec<PhotoId> = project.photos.iter().map(|p| p.id).collect();
    let before: Vec<FrameView> = (0..5)
        .map(|i| frame(&old[i], photo(p[i]), (0.1 * i as f64, 0.0, 0.1, 0.1)))
        .collect();
    let after = vec![before[1].clone(), before[3].clone()];
    let mut slots = Slots::default();
    let here = (1, project.spreads[1].id);
    slots.update(&before, here, false);
    let start = ids(&slots);
    slots.update(&after, here, false);
    let dying = slots.slots().iter().filter(|s| !s.live).count();
    assert_eq!(dying, 3);
    // Passado o esmaecimento, só elas saem, por remoção, da maior posição para a menor.
    assert_eq!(slots.sweep(), vec![4, 2, 0]);
    assert_eq!(ids(&slots), vec![start[1], start[3]]);
    assert!(slots.sweep().is_empty());
}

#[test]
fn mv_b25_a_foto_inteira_tem_a_proporcao_da_foto_e_a_mesma_regiao_da_impressao() {
    let project = album(4);
    for index in 0..project.spreads.len() {
        for f in view(&project, index) {
            let FrameKind::Photo { id, source, .. } = &f.kind else {
                continue;
            };
            let p = project.photo(*id).unwrap();
            let e = photo_extent(&f).unwrap();
            let aspect = f64::from(p.width_px) / f64::from(p.height_px);
            assert!((e.width / e.height - aspect).abs() < 1e-9, "proporção");
            // A parte da foto inteira que cai no quadro é a região da impressão.
            assert!(((f.rect.x - e.x) / e.width - source.x).abs() < 1e-9);
            assert!(((f.rect.y - e.y) / e.height - source.y).abs() < 1e-9);
            assert!((f.rect.width / e.width - source.width).abs() < 1e-9);
            assert!((f.rect.height / e.height - source.height).abs() < 1e-9);
        }
    }
}

#[test]
fn mv_b27_mudar_a_posicao_nao_troca_a_foto_da_vaga() {
    let mut project = album(4);
    let b = project.add_photo("b.jpg", 3000, 2000);
    let mut editor = editor_at(project, 1);
    let mut slots = Slots::default();
    slots.update(&current(&editor), place(&editor), false);
    let a = match &current(&editor)[0].kind {
        FrameKind::Photo { id, .. } => *id,
        _ => panic!("quadro de foto"),
    };
    let slot = slot_of(&slots, SlotContent::Photo(a)).id;
    assert!(editor.run(Command::NextLayout));
    slots.update(&current(&editor), place(&editor), false);
    // A mudou de lugar: a vaga é a mesma e a foto dela também.
    let same = slots.slots().iter().find(|s| s.id == slot).unwrap();
    assert_eq!(same.content, SlotContent::Photo(a));
    // Troco A por B nessa vaga: a foto da vaga muda.
    let at = same.frame.unwrap();
    assert!(editor.drop_photo(at, b));
    slots.update(&current(&editor), place(&editor), false);
    let now = slots.slots().iter().find(|s| s.id == slot).unwrap();
    assert_eq!(now.content, SlotContent::Photo(b));
    let s = editor.session().unwrap();
    let in_frame = &s.project().spreads[s.current()].frames[at].content;
    assert!(matches!(in_frame, FrameContent::Photo { photo, .. } if *photo == b));
}
