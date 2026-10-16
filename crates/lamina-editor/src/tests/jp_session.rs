//! Abrir, navegar, salvar, fechar e erros ao abrir (JP-B1, B3, B6, B7, B9, B11).

use lamina_core::{CoreError, FrameContent};
use lamina_export::ProjectFileError;

use super::common::{Base, file_name};
use crate::{Command, Editor, EditorError, FrameKind, Leave, Session, spread_view, view::Area};

#[test]
fn jp_b1_abrir_um_projeto_mostra_o_album() {
    let base = Base::with_photos(12, (600, 400));
    let file = base.save(base.project(3), "album/casamento.lamina");
    let session = Session::open(&file).unwrap();
    assert_eq!(session.current(), 0);
    assert!(!session.is_dirty());
    assert_eq!(session.title(), "casamento");
    assert_eq!(
        session.counter(),
        "3 lâminas (6 páginas) · 12 de 12 fotos usadas"
    );
}

#[test]
fn jp_b3_navegar_pelas_laminas() {
    let base = Base::with_photos(12, (600, 400));
    let mut editor = Editor::new();
    editor.adopt(base.project(3));
    let at = |e: &Editor| e.session().unwrap().current();
    assert!(editor.run(Command::NextSpread));
    assert_eq!(at(&editor), 1);
    assert!(editor.run(Command::NextSpread));
    assert_eq!(at(&editor), 2);
    assert!(!editor.available(Command::NextSpread));
    assert!(!editor.run(Command::NextSpread));
    assert_eq!(at(&editor), 2);
    assert!(editor.run(Command::FirstSpread));
    assert_eq!(at(&editor), 0);
    assert!(editor.run(Command::LastSpread));
    assert_eq!(at(&editor), 2);
    // Clique na lâmina 2 da faixa.
    assert!(editor.session_mut().unwrap().go_to(1));
    assert_eq!(at(&editor), 1);
    assert!(!editor.session_mut().unwrap().go_to(3), "não existe");
}

#[test]
fn jp_b6_salvar_e_salvar_como() {
    let base = Base::with_photos(12, (600, 400));
    let mut session = Session::new_unsaved(base.project(3));
    assert!(matches!(session.save(), Err(EditorError::NoFile)));
    for folder in ["album", "outra"] {
        std::fs::create_dir_all(base.path(folder)).unwrap();
    }
    session
        .save_as(&base.path("album/casamento.lamina"))
        .unwrap();
    let text = std::fs::read_to_string(base.path("album/casamento.lamina")).unwrap();
    assert_eq!(text.matches("\"../fotos/f").count(), 12, "{text}");
    assert!(!session.is_dirty());
    session.save_as(&base.path("outra/copia.lamina")).unwrap();
    let copy = std::fs::read_to_string(base.path("outra/copia.lamina")).unwrap();
    assert_eq!(copy.matches("\"../fotos/f").count(), 12);
    assert_eq!(file_name(session.file().unwrap()), "copia.lamina");
}

#[test]
fn jp_b7_fechar_com_alteracoes_nao_salvas() {
    let base = Base::with_photos(8, (600, 400));
    let mut editor = Editor::new();
    assert!(!editor.needs_confirmation(), "sem projeto");
    editor.adopt(base.project(2));
    assert!(editor.needs_confirmation());
    assert!(!editor.leave(Leave::Cancel, None).unwrap());
    assert!(editor.needs_confirmation(), "Cancelar mantém tudo");
    assert!(editor.leave(Leave::DontSave, None).unwrap());
    assert!(matches!(
        editor.leave(Leave::Save, None),
        Err(EditorError::NoFile)
    ));
    std::fs::create_dir_all(base.path("album")).unwrap();
    let target = base.path("album/casamento.lamina");
    assert!(editor.leave(Leave::Save, Some(&target)).unwrap());
    assert!(target.exists());
    assert!(!editor.needs_confirmation(), "salvo: nada a perguntar");
}

#[test]
fn jp_b9_foto_ausente_ao_abrir() {
    let base = Base::with_photos(12, (600, 400));
    let file = base.save(base.project(3), "album/casamento.lamina");
    std::fs::remove_file(base.path("fotos/f05.jpg")).unwrap();
    let session = Session::open(&file).unwrap();
    assert_eq!(
        session.missing_message().as_deref(),
        Some("1 foto não foi encontrada: f05.jpg.")
    );
    let gone = session.missing()[0];
    let marked = (0..session.spread_count())
        .filter_map(|i| spread_view(session.project(), i, AREA, session.missing()))
        .flat_map(|v| v.frames)
        .filter(|f| matches!(f.kind, FrameKind::Photo { id, missing: true, .. } if id == gone))
        .count();
    assert_eq!(marked, 1);
}

const AREA: Area = Area {
    width: 1200.0,
    height: 700.0,
    margin: 24.0,
};

#[test]
fn varias_fotos_ausentes_no_aviso() {
    let base = Base::with_photos(12, (600, 400));
    let file = base.save(base.project(3), "album/casamento.lamina");
    for i in 0..7 {
        std::fs::remove_file(base.path(&format!("fotos/f{i:02}.jpg"))).unwrap();
    }
    let session = Session::open(&file).unwrap();
    let message = session.missing_message().unwrap();
    assert!(
        message.starts_with("7 fotos não foram encontradas: "),
        "{message}"
    );
    assert!(message.ends_with(" e mais 2."), "{message}");
}

#[test]
fn jp_b11_arquivo_invalido_nao_derruba_a_sessao() {
    let base = Base::with_photos(12, (600, 400));
    let file = base.save(base.project(3), "album/casamento.lamina");
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    assert!(editor.run(Command::NextSpread));
    let broken = base.path("album/quebrado.lamina");
    std::fs::write(&broken, "{ isto não é JSON").unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    let future_text = text.replacen("\"schema_version\": 2", "\"schema_version\": 99", 1);
    assert_ne!(text, future_text, "o arquivo grava a versão do formato");
    let future = base.path("album/futuro.lamina");
    std::fs::write(&future, future_text).unwrap();

    let err = editor.open(&broken).unwrap_err();
    assert!(
        matches!(
            err,
            EditorError::File(ProjectFileError::Core(CoreError::Json(_)))
        ),
        "{err}"
    );
    let err = editor.open(&future).unwrap_err();
    assert!(
        matches!(
            err,
            EditorError::File(ProjectFileError::Core(CoreError::UnsupportedSchema {
                found: 99,
                ..
            }))
        ),
        "{err}"
    );
    let session = editor.session().unwrap();
    assert_eq!(file_name(session.file().unwrap()), "casamento.lamina");
    assert_eq!(session.current(), 1);
}

#[test]
fn quadro_vazio_aparece_vazio() {
    let base = Base::with_photos(4, (600, 400));
    let mut project = base.project(1);
    project.spreads[0].frames[0].content = FrameContent::Empty;
    let view = spread_view(&project, 0, AREA, &[]).unwrap();
    assert_eq!(view.frames[0].kind, FrameKind::Empty);
}
