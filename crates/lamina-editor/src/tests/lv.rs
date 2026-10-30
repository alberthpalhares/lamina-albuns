//! Cenários da spec `leveza` (LV) na sessão: o álbum compartilhado, "tem alteração?", a
//! pré-impressão por lâmina, a conferência das fotos que faltam e a lista de fotos.

use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use lamina_core::testing::{TRIES, budget};
use lamina_core::{AlbumSpec, Dpi, FrameContent, Mm, Project, RelRect, Rgb, TextBox};
use lamina_export::preflight;

use crate::{Session, same_photo_list};

fn spec() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
}

fn rect() -> RelRect {
    RelRect::new(0.1, 0.1, 0.5, 0.5).unwrap()
}

fn text(s: &str) -> FrameContent {
    FrameContent::Text(TextBox {
        text: s.to_owned(),
        ..TextBox::new(Rgb::BLACK)
    })
}

/// `spreads` lâminas, com um texto em cada e `photos` fotos no álbum.
fn album(spreads: usize, photos: usize) -> Project {
    let mut project = Project::new("teste", spec());
    for n in 0..photos {
        project.add_photo(format!("f{n}.jpg"), 6000, 4000);
    }
    for _ in 0..spreads {
        let id = project.add_spread();
        project.add_frame(id, rect(), text("Ana & Pedro")).unwrap();
    }
    project
}

fn write_text(session: &mut Session, spread: usize, value: &str) {
    let _ = session.edit::<Infallible>("Texto", |p| {
        if let FrameContent::Text(t) = &mut p.spreads[spread].frames[0].content {
            value.clone_into(&mut t.text);
        }
        Ok(())
    });
}

#[test]
fn lv_b2_editar_compartilha_o_album_em_vez_de_copiar() {
    let mut session = Session::new_unsaved(album(2, 3));
    let before = session.project_shared();
    assert!(Arc::ptr_eq(&before, &session.project_shared()));
    write_text(&mut session, 0, "Outro");
    let after = session.project_shared();
    assert!(!Arc::ptr_eq(&before, &after));
    assert!(Arc::ptr_eq(&after, &session.project_shared()));
    assert!(session.undo().is_some());
    assert!(Arc::ptr_eq(&before, &session.project_shared()));
}

#[test]
fn lv_b3_tem_alteracao_acompanha_editar_desfazer_e_refazer() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::new_unsaved(album(2, 1));
    assert!(session.is_dirty());
    session.save_as(&dir.path().join("a.lamina")).unwrap();
    assert!(!session.is_dirty());
    write_text(&mut session, 0, "Outro");
    assert!(session.is_dirty());
    assert!(session.undo().is_some());
    assert!(!session.is_dirty());
    assert!(session.redo().is_some());
    assert!(session.is_dirty());
    // Voltar à mão ao texto salvo também conta como sem alteração.
    write_text(&mut session, 0, "Ana & Pedro");
    assert!(!session.is_dirty());
    let mut prefs = session.export_prefs();
    prefs.pages = !prefs.pages;
    session.set_export_prefs(prefs);
    assert!(session.is_dirty());
}

#[test]
fn lv_b4_pre_impressao_so_da_lamina_editada() {
    let mut session = Session::new_unsaved(album(5, 1));
    let before = session.project_shared();
    // Um texto grande demais para o quadro gera um aviso, só na lâmina 3.
    write_text(&mut session, 2, &"muito texto ".repeat(400));
    let now = session.project_shared();
    assert_eq!(now.spreads_changed_since(&before), Some(vec![2]));
    let all: Vec<usize> = (0..now.spreads.len()).collect();
    assert_eq!(session.warnings_for(2).len(), preflight(&now, &[2]).len());
    assert!(!session.warnings_for(2).is_empty());
    let mut counted = 0;
    for &i in &all {
        counted += session.warnings_for(i).len();
    }
    assert_eq!(counted, preflight(&now, &all).len());
}

#[test]
fn lv_b4_mudanca_no_album_todo_confere_tudo() {
    let session = Session::new_unsaved(album(3, 1));
    let before = session.project_shared();
    let mut other = (*before).clone();
    other.add_spread();
    assert_eq!(other.spreads_changed_since(&before), None);
    let mut renamed = (*before).clone();
    renamed.photos[0].width_px = 10;
    assert_eq!(renamed.spreads_changed_since(&before), None);
}

#[test]
fn lv_b5_rapido_digitar_com_vinte_textos() {
    let mut session = Session::new_unsaved(album(20, 1000));
    let mut best = std::time::Duration::MAX;
    let mut letters = String::new();
    for _ in 0..TRIES {
        letters.push('a');
        let start = Instant::now();
        session
            .edit_merging::<Infallible>("Texto", "t", |p| {
                if let FrameContent::Text(t) = &mut p.spreads[0].frames[0].content {
                    t.text.clone_from(&letters);
                }
                Ok(())
            })
            .unwrap();
        let _ = session.is_dirty();
        best = best.min(start.elapsed());
        if best < budget(10) {
            break;
        }
    }
    assert!(best < budget(10), "digitar levou {best:?}");
}

fn with_file(dir: &Path) -> Project {
    let file = dir.join("f.jpg");
    std::fs::write(&file, b"x").unwrap();
    let mut project = Project::new("teste", spec());
    project.add_photo(file, 100, 100);
    project.add_spread();
    project
}

#[test]
fn lv_b8_desfazer_um_ajuste_nao_confere_o_disco() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::new_unsaved(with_file(dir.path()));
    assert!(session.missing().is_empty());
    std::fs::remove_file(dir.path().join("f.jpg")).unwrap();
    let _ = session.edit::<Infallible>("Brilho", |p| {
        p.photos[0].adjust.brightness = 10;
        Ok(())
    });
    assert!(session.undo().is_some());
    // O arquivo sumiu, mas ninguém olhou: nenhum caminho mudou.
    assert!(session.missing().is_empty());
}

#[test]
fn lv_b9_trocar_o_caminho_confere_o_disco() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::new_unsaved(with_file(dir.path()));
    let _ = session.edit::<Infallible>("Reencontrar", |p| {
        p.photos[0].path = dir.path().join("outra.jpg");
        Ok(())
    });
    std::fs::remove_file(dir.path().join("f.jpg")).unwrap();
    assert!(session.undo().is_some());
    assert_eq!(session.missing().len(), 1);
}

#[test]
fn lv_b10_mexer_so_num_quadro_nao_refaz_a_lista_de_fotos() {
    let mut project = album(2, 2);
    let photo = project.photos[0].id;
    let spread = project.spreads[0].id;
    let content = FrameContent::Photo {
        photo,
        crop: lamina_core::Crop::default(),
    };
    project.add_frame(spread, rect(), content).unwrap();
    let mut moved = project.clone();
    moved.spreads[0].frames[1].rect = RelRect::new(0.2, 0.2, 0.3, 0.3).unwrap();
    assert!(same_photo_list(&project, &moved));
    let mut other_spread = project.clone();
    let frame = other_spread.spreads[0].frames.remove(1);
    other_spread.spreads[1].frames.push(frame);
    assert!(!same_photo_list(&project, &other_spread));
    let mut renamed = project.clone();
    renamed.photos[1].path = "g.jpg".into();
    assert!(!same_photo_list(&project, &renamed));
}

#[test]
fn lv_b17_pintar_de_novo_nao_le_o_disco_e_o_cache_tem_limite() {
    use lamina_render::PhotoSource;

    use crate::motion::Tier;
    use crate::{ROLL_IMAGES_BYTES, RollImages, RollSource};

    let base = super::common::Base::with_photos(2, (600, 400));
    let project = base.project(1);
    let cache = base.cache();
    let images = Arc::new(RollImages::new(ROLL_IMAGES_BYTES));
    let source = RollSource::new(&project, 0, &cache, Tier::Small).sharing(images.clone());
    let frame = project.spreads[0]
        .frames
        .iter()
        .find_map(|f| match &f.content {
            FrameContent::Photo { photo, .. } => Some(*photo),
            _ => None,
        });
    let photo = project.photo(frame.unwrap()).unwrap();
    let first = source.load(photo, (100, 100)).unwrap();
    assert!(images.bytes() > 0);
    // O arquivo sumiu do cache: a segunda pintura nem olha o disco.
    std::fs::remove_file(cache.thumbnail(photo).unwrap()).unwrap();
    let second = source.load(photo, (100, 100)).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    images.clear();
    assert_eq!(images.bytes(), 0);
    // Limite menor que uma imagem: nada fica guardado.
    let tiny = Arc::new(RollImages::new(10));
    let other = RollSource::new(&project, 0, &base.cache(), Tier::Small).sharing(tiny.clone());
    let _ = other.load(photo, (100, 100));
    assert_eq!(tiny.bytes(), 0);
}
