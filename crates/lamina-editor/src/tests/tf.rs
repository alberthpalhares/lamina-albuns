//! Cenários da spec `tirar-fotos-do-album` (TF): tirar fotos pela grade (com e sem lâminas), o
//! Ctrl+Z, o arquivo no disco, o Delete na lâmina e o "Mostrar no Explorador".

use lamina_core::{FrameContent, PhotoId, Project};

use super::tl_common::{editor_at, three_spreads};
use crate::leaving::{leave_label, leave_question, remove_from_album};
use crate::reveal::{System, reveal_command};
use crate::{Command, Editor, EditorError};

fn used(project: &Project, id: PhotoId) -> bool {
    project.spreads.iter().any(|s| {
        s.frames
            .iter()
            .any(|f| matches!(f.content, FrameContent::Photo { photo, .. } if photo == id))
    })
}

fn in_spread(project: &Project, index: usize) -> Vec<PhotoId> {
    project.spreads[index]
        .frames
        .iter()
        .filter_map(|f| match f.content {
            FrameContent::Photo { photo, .. } => Some(photo),
            _ => None,
        })
        .collect()
}

/// O álbum de três lâminas mais uma foto fora das lâminas.
fn with_loose() -> (Project, PhotoId) {
    let mut project = three_spreads();
    let loose = project.add_photo("solta.jpg", 3000, 2000);
    (project, loose)
}

fn photo_ids(editor: &Editor) -> Vec<PhotoId> {
    let s = editor.session().unwrap();
    s.project().photos.iter().map(|p| p.id).collect()
}

#[test]
fn tf_b1_tirar_uma_foto_que_nao_esta_em_lamina() {
    let (project, loose) = with_loose();
    let mut editor = editor_at(project.clone(), 0);
    assert_eq!(editor.photos_in_spreads(&[loose]), 0);
    assert_eq!(leave_question(0), None, "sem pergunta");
    assert!(editor.remove_from_album(&[loose]));
    assert!(!photo_ids(&editor).contains(&loose));
    let s = editor.session().unwrap();
    assert_eq!(s.undo_label(), Some("Tirar foto do álbum"));
    assert_eq!(s.project().spreads, project.spreads, "as lâminas não mudam");
}

#[test]
fn tf_b2_tirar_fotos_usadas_pede_confirmacao() {
    let (project, loose) = with_loose();
    let a = in_spread(&project, 0)[0];
    let b = in_spread(&project, 1)[0];
    let editor = editor_at(project, 0);
    assert_eq!(editor.photos_in_spreads(&[a, b, loose]), 2);
    assert_eq!(
        leave_question(2).unwrap(),
        "2 fotos estão em lâminas. Elas saem das lâminas e o layout delas é refeito. O arquivo \
         continua no computador."
    );
    assert_eq!(leave_label(3), "Tirar 3 fotos do álbum");
}

#[test]
fn tf_b3_as_laminas_sao_refeitas() {
    let project = three_spreads();
    let before = in_spread(&project, 1);
    let mut editor = editor_at(project, 0);
    assert!(editor.remove_from_album(&[before[0]]));
    let s = editor.session().unwrap();
    assert_eq!(
        in_spread(s.project(), 1),
        before[1..],
        "três fotos, na mesma ordem"
    );
    let empty = s.project().spreads[1]
        .frames
        .iter()
        .any(|f| f.content == FrameContent::Empty);
    assert!(!empty, "sem quadro vazio");
    s.project().validate().unwrap();
}

#[test]
fn tf_b4_um_ctrl_z_desfaz_tudo() {
    let (project, loose) = with_loose();
    let used_one = in_spread(&project, 2)[1];
    let mut editor = editor_at(project.clone(), 0);
    assert!(editor.remove_from_album(&[used_one, loose]));
    assert!(editor.run(Command::Undo));
    let s = editor.session().unwrap();
    assert_eq!(
        s.project().photos,
        project.photos,
        "na mesma posição da grade"
    );
    assert_eq!(s.project().spreads, project.spreads, "as lâminas idênticas");
}

#[test]
fn tf_b5_o_arquivo_fica_no_disco() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("foto.jpg");
    std::fs::write(&file, b"bytes da foto").unwrap();
    let mut project = three_spreads();
    let id = project.add_photo(file.to_string_lossy().to_string(), 3000, 2000);
    remove_from_album(&mut project, &[id]).unwrap();
    assert!(!project.photos.iter().any(|p| p.id == id));
    assert_eq!(std::fs::read(&file).unwrap(), b"bytes da foto");
}

#[test]
fn tf_b6_delete_com_o_foco_na_lamina_nao_mexe_na_grade() {
    let project = three_spreads();
    let first = in_spread(&project, 1)[0];
    let mut editor = editor_at(project, 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::ClearFrame));
    let s = editor.session().unwrap();
    assert!(!used(s.project(), first), "a foto sai do quadro");
    assert!(photo_ids(&editor).contains(&first), "e continua na grade");
}

#[test]
fn tf_b7_mostrar_no_explorador() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("foto.jpg");
    std::fs::write(&file, b"x").unwrap();
    let shown = file.display().to_string();
    let (program, args) = reveal_command(&file, System::Windows).unwrap();
    assert_eq!(
        (program.as_str(), args),
        ("explorer", vec![format!("/select,{shown}")])
    );
    let (program, args) = reveal_command(&file, System::Mac).unwrap();
    assert_eq!(
        (program.as_str(), args),
        ("open", vec!["-R".to_owned(), shown])
    );
    let (program, args) = reveal_command(&file, System::Linux).unwrap();
    let folder = dir.path().display().to_string();
    assert_eq!((program.as_str(), args), ("xdg-open", vec![folder]));
    assert_eq!(System::Windows.reveal_label(), "Mostrar no Explorador");
    assert_eq!(System::Mac.reveal_label(), "Mostrar no Finder");
}

#[test]
fn tf_b8_arquivo_sumido() {
    use crate::Human as _;
    let dir = tempfile::tempdir().unwrap();
    let gone = dir.path().join("movida.jpg");
    let err = reveal_command(&gone, System::Windows).unwrap_err();
    assert!(matches!(err, EditorError::PhotoFileMissing(_)));
    let message = err.human();
    assert_eq!(message.title, "Não achei esta foto no computador.");
    assert!(message.what_to_do.contains("Reencontrar fotos"));
}

#[test]
fn tf_b9_foto_repetida_sai_de_todas_as_laminas() {
    let mut project = three_spreads();
    let repeated = in_spread(&project, 0)[0];
    // A mesma foto também no primeiro quadro da lâmina 2 (a terceira).
    if let FrameContent::Photo { photo, .. } = &mut project.spreads[2].frames[0].content {
        *photo = repeated;
    }
    let mut editor = editor_at(project, 0);
    assert!(editor.remove_from_album(&[repeated]));
    let s = editor.session().unwrap();
    assert!(!used(s.project(), repeated), "sai das lâminas 1 e 3");
    s.project().validate().unwrap();
}
