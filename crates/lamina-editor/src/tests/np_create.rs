//! Cenários da spec `novo-projeto` (NP) da criação: aparência em todas as lâminas, começar sem
//! fotos e criar já gravando, sem substituir arquivo nenhum (NP-B11 a NP-B13).

use std::path::Path;

use lamina_core::{AlbumLook, Border, CropLevel, FrameContent, Mm, Mounting, Project, Rgb};

use super::common::Base;
use crate::new_project::{
    AlbumStyle, can_write, documents_dir, empty_album, free_file, suggested_folder,
};
use crate::{Editor, create_album};

fn style() -> AlbumStyle {
    AlbumStyle {
        mounting: Mounting {
            crop: CropLevel::Whole,
            mosaics: false,
            ..Mounting::default()
        },
        look: AlbumLook {
            background: Rgb {
                r: 128,
                g: 128,
                b: 128,
            },
            border: Border {
                width: Mm(0.5),
                color: Rgb::WHITE,
            },
            radius: Mm(2.0),
        },
        gap: Mm(4.0),
    }
}

#[test]
fn np_b11_aparencia_em_todas_as_laminas() {
    let base = Base::with_photos(12, (600, 400));
    let mut new = base.new_album(Some(3));
    new.style = style();
    let (built, _) = create_album(&new, &base.cache(), &|_| {}).unwrap();
    let project = built.project;
    assert_eq!(project.spreads.len(), 3);
    assert_eq!(project.gap, Mm(4.0));
    assert_eq!(project.mounting, style().mounting);
    for spread in &project.spreads {
        assert_eq!(
            spread.background,
            Rgb {
                r: 128,
                g: 128,
                b: 128
            }
        );
        assert_eq!(spread.background_right, None);
        assert_eq!(project.spread_gap(spread), Mm(4.0));
        for frame in &spread.frames {
            if matches!(frame.content, FrameContent::Photo { .. }) {
                assert_eq!(frame.border, style().look.border);
                assert_eq!(frame.radius, Mm(2.0));
            }
        }
    }
}

#[test]
fn np_b12_comecar_sem_fotos() {
    let base = Base::with_photos(1, (600, 400));
    let mut new = base.new_album(None);
    new.name = Some("Ana".into());
    new.style = style();
    let project = empty_album(&new);
    let file = base.path("albuns/Ana.lamina");
    let mut editor = Editor::new();
    let saved = editor.adopt_saved(project, &file).unwrap();
    assert!(saved.is_file());
    let back = Project::from_json(&std::fs::read_to_string(&saved).unwrap()).unwrap();
    assert_eq!(back.name, "Ana");
    assert_eq!(back.spec, new.spec);
    assert_eq!(back.mounting, style().mounting);
    assert_eq!(back.look, style().look);
    assert_eq!(back.gap, Mm(4.0));
    assert_eq!((back.photos.len(), back.spreads.len()), (0, 0));
}

#[test]
fn np_b13_criar_ja_grava_e_nunca_substitui() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Ana.lamina"), "{}").unwrap();
    let file = free_file(dir.path(), "Ana");
    assert_eq!(file, dir.path().join("Ana (2).lamina"));
    std::fs::write(&file, "{}").unwrap();
    assert_eq!(
        free_file(dir.path(), "Ana"),
        dir.path().join("Ana (3).lamina")
    );
    assert_eq!(free_file(dir.path(), "Bia"), dir.path().join("Bia.lamina"));

    let base = Base::with_photos(1, (600, 400));
    let mut new = base.new_album(None);
    new.name = Some("Ana".into());
    let mut editor = Editor::new();
    let target = free_file(dir.path(), "Ana");
    let saved = editor.adopt_saved(empty_album(&new), &target).unwrap();
    assert_eq!(saved, std::path::absolute(&target).unwrap());
    assert_eq!(
        editor.recents().list()[0].path,
        saved,
        "no topo dos recentes"
    );
    assert!(!editor.needs_confirmation(), "já gravado");
    // O "Ana.lamina" que já existia continua igual.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("Ana.lamina")).unwrap(),
        "{}"
    );
}

#[test]
fn onde_salvar_sugere_a_pasta_das_fotos_ou_a_ultima_ou_documentos() {
    let (photos, last, docs) = (Path::new("/f"), Path::new("/u"), Path::new("/d"));
    assert_eq!(suggested_folder(Some(photos), Some(last), docs), photos);
    assert_eq!(suggested_folder(None, Some(last), docs), last);
    assert_eq!(suggested_folder(None, None, docs), docs.join("Lamina"));
}

#[test]
fn pasta_que_ainda_nao_existe_vale_pela_de_cima() {
    let dir = tempfile::tempdir().unwrap();
    assert!(can_write(&dir.path().join("Lamina").join("Novo")));
    assert!(documents_dir().is_dir());
}
