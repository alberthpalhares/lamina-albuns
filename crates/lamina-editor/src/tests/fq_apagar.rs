//! Cenários da spec `fotos-nos-quadros` sobre apagar a foto (FQ-B3, FQ-B13 a FQ-B16): a foto sai
//! com o quadro e a lâmina ganha layout novo para as fotos que restaram.

use lamina_core::{FrameContent, PhotoId, Project, RelRect, Rgb, TextBox};

use super::tl_common::{editor_at, frames, three_spreads};
use crate::edits::{remove_photos, spread_photos};
use crate::{Command, Editor};

fn no_empty(project: &Project, index: usize) -> bool {
    project.spreads[index]
        .frames
        .iter()
        .all(|f| f.content != FrameContent::Empty)
}

/// A lâmina 1 com duas fotos em cada página, sem nada sobre a lombada.
fn two_pages() -> (Project, Vec<PhotoId>) {
    let mut project = three_spreads();
    let photos = spread_photos(&project, 1);
    let id = project.spreads[1].id;
    project.spreads[1].frames.clear();
    let rects = [(0.05, 0.1), (0.05, 0.55), (0.55, 0.1), (0.55, 0.55)];
    for ((x, y), photo) in rects.iter().zip(&photos) {
        let rect = RelRect::new(*x, *y, 0.4, 0.35).unwrap();
        let crop = lamina_core::Crop::default();
        let content = FrameContent::Photo {
            photo: *photo,
            crop,
        };
        project.add_frame(id, rect, content).unwrap();
    }
    (project, photos)
}

fn delete_frame(editor: &mut Editor, frame: usize) {
    editor.select_frame(Some(frame));
    assert!(editor.run(Command::ClearFrame));
}

#[test]
fn fq_b3_apagar_foto_tira_o_quadro_e_refaz_o_layout() {
    let project = three_spreads();
    let mut editor = editor_at(project.clone(), 1);
    assert!(
        !editor.available(Command::ClearFrame),
        "sem quadro selecionado"
    );
    let before: Vec<PhotoId> = frames(&editor).iter().map(|f| f.0).collect();
    delete_frame(&mut editor, 0);
    let after: Vec<PhotoId> = frames(&editor).iter().map(|f| f.0).collect();
    assert_eq!(after, before[1..], "as outras fotos ficam, na mesma ordem");
    let s = editor.session().unwrap();
    assert!(no_empty(s.project(), 1), "nenhum quadro vazio");
    s.project().validate().unwrap();
    assert!(editor.run(Command::Undo));
    let s = editor.session().unwrap();
    assert_eq!(
        s.project().spreads[1],
        project.spreads[1],
        "um Ctrl+Z volta tudo"
    );
}

#[test]
fn fq_b13_apagar_foto_de_uma_pagina_refaz_so_aquela_pagina() {
    let (project, photos) = two_pages();
    let right: Vec<_> = project.spreads[1].frames[2..].to_vec();
    let mut editor = editor_at(project, 1);
    delete_frame(&mut editor, 0);
    let s = editor.session().unwrap();
    let spread = &s.project().spreads[1];
    for f in &right {
        assert!(
            spread
                .frames
                .iter()
                .any(|g| g.rect == f.rect && g.content == f.content),
            "a página direita fica como estava"
        );
    }
    assert_eq!(spread_photos(s.project(), 1).len(), 3);
    assert!(!spread_photos(s.project(), 1).contains(&photos[0]));
    assert!(no_empty(s.project(), 1));
    let left = spread.frames.iter().filter(|f| f.rect.x() < 0.5).count();
    assert_eq!(left, 1, "a página esquerda ficou com a foto que restou");
}

#[test]
fn fq_b14_apagar_a_ultima_foto_deixa_a_lamina_em_branco() {
    let mut project = three_spreads();
    let spreads = project.spreads.len();
    let id = project.spreads[1].id;
    project.spreads[1].frames.truncate(1);
    let rect = RelRect::new(0.6, 0.1, 0.3, 0.1).unwrap();
    let text = FrameContent::Text(TextBox::new(Rgb::BLACK));
    project.add_frame(id, rect, text).unwrap();
    let mut editor = editor_at(project, 1);
    delete_frame(&mut editor, 0);
    let s = editor.session().unwrap();
    assert_eq!(
        s.project().spreads.len(),
        spreads,
        "a lâmina continua no álbum"
    );
    let left = &s.project().spreads[1].frames;
    assert_eq!(left.len(), 1, "só o texto fica");
    assert!(matches!(left[0].content, FrameContent::Text(_)));
}

#[test]
fn fq_b15_lamina_feita_a_mao_tambem_ganha_layout_novo() {
    let mut project = three_spreads();
    project.spreads[1].manual = true;
    let n = spread_photos(&project, 1).len();
    let mut editor = editor_at(project, 1);
    delete_frame(&mut editor, 1);
    let s = editor.session().unwrap();
    assert!(!s.project().spreads[1].manual, "a lâmina volta ao motor");
    assert_eq!(spread_photos(s.project(), 1).len(), n - 1);
    assert!(no_empty(s.project(), 1));
}

#[test]
fn fq_b16_apagar_varias_fotos_de_uma_vez() {
    let mut project = three_spreads();
    let mut left = spread_photos(&project, 1);
    let gone: Vec<PhotoId> = [0, 2]
        .iter()
        .filter_map(|f| crate::suggestions::photo_of(&project.spreads[1].frames[*f]))
        .collect();
    left.retain(|p| !gone.contains(p));
    remove_photos(&mut project, 1, &[0, 2]).unwrap();
    let mut now = spread_photos(&project, 1);
    now.sort_unstable_by_key(|p| left.iter().position(|q| q == p));
    assert_eq!(now, left);
    assert!(no_empty(&project, 1));
    project.validate().unwrap();
}

#[test]
fn fq_b16_apagar_foto_de_quadro_que_nao_existe_e_recusado() {
    let mut project = three_spreads();
    let before = project.clone();
    let r = remove_photos(&mut project, 1, &[9]);
    assert!(matches!(r, Err(crate::EditorError::NoFrame)));
    assert_eq!(project, before);
}
