//! O Delete em camadas e a área de cópia do modo organizar (spec EG, regras 11 e 12): EG-B18 e
//! EG-B19.

use lamina_core::{FrameContent, Rect, Rgb, TextBox};

use super::eg_imas::album_mm;
use super::tl_common::editor_at;
use crate::{Command, Editor};

fn close(a: Rect, b: Rect, shift: f64) -> bool {
    (a.x.get() + shift - b.x.get()).abs() < 1e-6
        && (a.y.get() + shift - b.y.get()).abs() < 1e-6
        && (a.width.get() - b.width.get()).abs() < 1e-6
        && (a.height.get() - b.height.get()).abs() < 1e-6
}

fn last_label(editor: &Editor) -> String {
    let s = editor.session().unwrap();
    let label = s.history_list().steps.last().unwrap().label.clone();
    label.split(" · ").next().unwrap_or_default().to_owned()
}

fn contents(editor: &Editor) -> Vec<FrameContent> {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()]
        .frames
        .iter()
        .map(|f| f.content.clone())
        .collect()
}

/// No modo, na lâmina 1, com dois quadros de foto e um texto selecionados.
fn two_photos_and_text() -> Editor {
    let mut project = album_mm(&[
        (20.0, 40.0, 40.0, 50.0),
        (80.0, 60.0, 50.0, 70.0),
        (150.0, 30.0, 60.0, 20.0),
    ]);
    project.spreads[1].frames[2].content = FrameContent::Text(TextBox::new(Rgb::BLACK));
    let mut editor = editor_at(project, 1);
    assert!(editor.run(Command::Organize));
    for f in 0..3 {
        editor.organize_click(f, f > 0, false);
    }
    editor
}

#[test]
fn eg_b18_delete_em_camadas() {
    let mut editor = two_photos_and_text();
    let photos = contents(&editor)
        .iter()
        .filter(|c| matches!(c, FrameContent::Photo { .. }))
        .count();
    assert_eq!(photos, 2, "o álbum de teste tem duas fotos selecionadas");
    let frames = contents(&editor).len();

    // Com fotos na seleção, elas saem com os quadros e o layout é refeito (spec FQ, regra 3).
    assert!(editor.run(Command::ClearFrame));
    assert_eq!(last_label(&editor), "Tirar 2 fotos");
    let now = contents(&editor);
    assert_eq!(now.len(), frames - 2, "as fotos saem com os quadros");
    assert!(matches!(now[0], FrameContent::Text(_)), "o texto fica");
    assert!(editor.selected_frames().is_empty());

    // Sem fotos na seleção, os quadros são apagados.
    editor.organize_click(0, false, false);
    assert!(editor.run(Command::ClearFrame));
    assert!(last_label(&editor).starts_with("Apagar"));
    assert_eq!(contents(&editor).len(), frames - 3);
    assert!(!editor.available(Command::ClearFrame));
}

#[test]
fn eg_b19_copiar_e_colar() {
    let mut editor = two_photos_and_text();
    editor.organize_click(0, false, false);
    editor.organize_click(1, true, false);
    let copied = [editor.frames_mm()[0], editor.frames_mm()[1]];
    assert!(
        !editor.available(Command::PasteFrames),
        "colar sem nada copiado fica indisponível"
    );
    assert!(editor.run(Command::CopyFrames));

    // Na outra lâmina: na mesma posição, selecionados, com "Colar 2 quadros".
    assert!(editor.session_mut().unwrap().go_to(3));
    let before = contents(&editor).len();
    assert!(editor.run(Command::PasteFrames));
    assert_eq!(last_label(&editor), "Colar 2 quadros");
    let f = editor.frames_mm();
    assert_eq!(f.len(), before + 2);
    assert!(close(copied[0], f[before], 0.0) && close(copied[1], f[before + 1], 0.0));
    assert_eq!(editor.selected_frames(), vec![before, before + 1]);
    assert!(editor.session().unwrap().project().spreads[3].manual);

    // Na mesma lâmina: 5 mm deslocados.
    assert!(editor.session_mut().unwrap().go_to(1));
    let before = contents(&editor).len();
    assert!(editor.run(Command::PasteFrames));
    let f = editor.frames_mm();
    assert!(close(copied[0], f[before], 5.0) && close(copied[1], f[before + 1], 5.0));
}

#[test]
fn eg_b19_duplicar_nao_mexe_na_area_de_copia() {
    let mut editor = two_photos_and_text();
    editor.organize_click(2, false, false);
    let text = editor.frames_mm()[2];
    assert!(editor.run(Command::DuplicateFrames));
    assert_eq!(last_label(&editor), "Duplicar quadro");
    let f = editor.frames_mm();
    assert!(close(text, f[f.len() - 1], 5.0));
    assert!(
        !editor.available(Command::PasteFrames),
        "Duplicar não copia"
    );
}

#[test]
fn eg_b19_copiar_fora_do_modo_usa_o_quadro_selecionado() {
    let project = album_mm(&[(20.0, 40.0, 40.0, 50.0), (80.0, 60.0, 50.0, 70.0)]);
    let mut editor = editor_at(project, 1);
    editor.organize_click(1, false, false);
    assert!(editor.run(Command::CopyFrames));
    assert!(editor.run(Command::PasteFrames));
    assert_eq!(last_label(&editor), "Colar quadro");
    assert_eq!(editor.selected_frames(), vec![2]);
}
