//! Cenários da spec `limpar-as-laminas` (LL): tirar as fotos de todas as lâminas e apagar todas
//! as lâminas, cada uma num passo só do histórico.

use lamina_core::{FrameContent, PageSide, Project};

use super::tl_common::{editor_at, three_spreads};
use crate::photos::used_photos;
use crate::{Command, Editor, TextChange};

fn project(editor: &Editor) -> &Project {
    editor.session().unwrap().project()
}

/// Três lâminas com fotos e um texto na lâmina 2.
fn with_text() -> Editor {
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.new_text(Some(PageSide::Left)));
    assert!(editor.change_text(TextChange::Text("Ana".into())));
    assert!(editor.session_mut().unwrap().go_to(0));
    editor
}

/// O rótulo inteiro do último passo do histórico.
fn last_label(editor: &Editor) -> String {
    let list = editor.session().unwrap().history_list();
    list.steps.last().unwrap().label.clone()
}

#[test]
fn ll_b1_tira_todas_as_fotos_num_passo_so() {
    let mut editor = with_text();
    let frames: Vec<usize> = project(&editor)
        .spreads
        .iter()
        .map(|s| s.frames.len())
        .collect();
    assert!(used_photos(project(&editor)) > 0);
    assert!(editor.run(Command::ClearAllPhotos));
    let p = project(&editor);
    assert!(
        p.spreads
            .iter()
            .flat_map(|s| &s.frames)
            .all(|f| !matches!(f.content, FrameContent::Photo { .. })),
        "nenhum quadro tem foto"
    );
    let after: Vec<usize> = p.spreads.iter().map(|s| s.frames.len()).collect();
    assert_eq!(after, frames, "os quadros continuam lá, vazios");
    assert!(
        p.spreads[1]
            .frames
            .iter()
            .any(|f| matches!(&f.content, FrameContent::Text(t) if t.text == "Ana")),
        "o texto fica"
    );
    assert_eq!(used_photos(p), 0, "todas as fotos ficam não usadas");
    assert_eq!(last_label(&editor), "Tirar as fotos · todas as lâminas");
    assert_eq!(
        editor.session().unwrap().current(),
        0,
        "a lâmina atual não muda"
    );
}

#[test]
fn ll_b2_ctrl_z_devolve_tudo() {
    let mut editor = with_text();
    let before = project(&editor).clone();
    assert!(editor.run(Command::ClearAllPhotos));
    assert!(editor.run(Command::Undo));
    assert_eq!(project(&editor), &before, "fotos e recortes de antes");
}

#[test]
fn ll_b3_sem_fotos_o_item_fica_apagado() {
    let mut editor = with_text();
    let photos = crate::photos::used_photos(project(&editor));
    assert_eq!(
        editor.clear_counts(),
        (photos, 3),
        "a pergunta conta fotos diferentes e lâminas"
    );
    assert!(editor.available(Command::ClearAllPhotos));
    assert!(editor.run(Command::ClearAllPhotos));
    assert!(
        !editor.available(Command::ClearAllPhotos),
        "sem fotos, apagado"
    );
    assert_eq!(editor.clear_counts(), (0, 3));
}

#[test]
fn ll_b4_apagar_todas_as_laminas() {
    let mut editor = with_text();
    let before = project(&editor).clone();
    assert!(editor.session_mut().unwrap().go_to(2));
    assert!(editor.run(Command::DeleteAllSpreads));
    let p = project(&editor);
    assert!(p.spreads.is_empty(), "o álbum fica sem lâminas");
    assert_eq!(p.photos, before.photos, "as fotos continuam importadas");
    assert_eq!(used_photos(p), 0);
    assert_eq!(last_label(&editor), "Apagar · todas as lâminas");
    assert_eq!(editor.session().unwrap().current(), 0);
    assert!(editor.run(Command::Undo));
    assert_eq!(project(&editor), &before, "Ctrl+Z devolve as 3 lâminas");
}

#[test]
fn ll_b5_sem_laminas_apagar_fica_apagado() {
    let mut editor = with_text();
    assert!(editor.available(Command::DeleteAllSpreads));
    assert_eq!(editor.clear_counts().1, 3);
    assert!(editor.run(Command::DeleteAllSpreads));
    assert!(
        !editor.available(Command::DeleteAllSpreads),
        "sem lâminas, apagado"
    );
    assert!(!editor.available(Command::ClearAllPhotos));
    assert_eq!(editor.clear_counts(), (0, 0));
}
