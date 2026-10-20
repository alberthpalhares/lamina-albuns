//! Cenários da spec `fotos-no-album` (IM), parte 2: fotos soltas numa lâmina. Na área de lâmina
//! nova ou entre lâminas, viram uma lâmina montada; fora dos quadros, entram na lâmina atual; sobre
//! um quadro, tudo como antes (IM-B7 a IM-B11 e IM-B14).

use std::path::PathBuf;

use lamina_core::{CropLevel, FrameContent, Mounting, PhotoId, Project};
use lamina_media::{CatalogEntry, ImportReport};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::edits::{layout_with, spread_photos};
use crate::{DropPlace, Editor};

/// Um álbum de `spreads` lâminas com 4 fotos em cada.
fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

fn photos(project: &Project, n: usize) -> Vec<PhotoId> {
    project.photos.iter().take(n).map(|p| p.id).collect()
}

fn last_label(editor: &Editor) -> String {
    let history = editor.session().unwrap().history_list();
    history.steps.last().unwrap().label.clone()
}

#[test]
fn im_b7_varias_fotos_na_area_de_lamina_nova() {
    let project = album(6);
    let five = photos(&project, 5);
    let mut expected = project.clone();
    crate::spreads::insert_spread(&mut expected, 5);
    layout_with(&mut expected, 6, &five).unwrap();
    let mut editor = editor_at(project, 2);
    assert!(editor.drop_photos(DropPlace::NewSpread(6), &five));
    let after = editor.session().unwrap().project();
    assert_eq!(after.spreads.len(), 7);
    assert_eq!(spread_photos(after, 6), five);
    let rects = |p: &Project| {
        p.spreads[6]
            .frames
            .iter()
            .map(|f| f.rect)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        rects(after),
        rects(&expected),
        "montada como \"Montar lâmina\" faria"
    );
    assert_eq!(
        editor.session().unwrap().current(),
        6,
        "a vista vai até ela"
    );
    assert!(
        last_label(&editor).starts_with("Nova lâmina com 5 fotos"),
        "{}",
        last_label(&editor)
    );
}

#[test]
fn im_b8_soltar_entre_laminas() {
    let project = album(5);
    let ids: Vec<_> = project.spreads.iter().map(|s| s.id).collect();
    let four = photos(&project, 4);
    let mut editor = editor_at(project, 0);
    // Entre as lâminas 2 e 3: o vão 2 (contando de 0 antes da primeira).
    assert!(editor.drop_photos(DropPlace::NewSpread(2), &four));
    let after = editor.session().unwrap().project();
    assert_eq!(after.spreads.len(), 6);
    assert_eq!(spread_photos(after, 2), four, "a lâmina nova é a 3");
    let moved: Vec<_> = after.spreads.iter().map(|s| s.id).collect();
    assert_eq!(&moved[..2], &ids[..2]);
    assert_eq!(
        &moved[3..],
        &ids[2..],
        "as antigas 3 em diante andam uma posição"
    );
}

#[test]
fn im_b9_soltar_na_lamina_atual_fora_de_quadro() {
    let mut project = album(5);
    // A lâmina 4 com 3 fotos.
    let three = spread_photos(&project, 3)[..3].to_vec();
    layout_with(&mut project, 3, &three).unwrap();
    let extra = spread_photos(&project, 0)[..2].to_vec();
    let mut editor = editor_at(project, 3);
    assert!(editor.drop_photos(DropPlace::Spread, &extra));
    let after = editor.session().unwrap().project();
    let mut want = three.clone();
    want.extend(&extra);
    assert_eq!(
        spread_photos(after, 3),
        want,
        "5 fotos num layout do gerador"
    );
    assert_eq!(after.spreads[3].frames.len(), 5);
    assert!(
        last_label(&editor).starts_with("Acrescentar 2 fotos"),
        "{}",
        last_label(&editor)
    );
    assert!(last_label(&editor).contains("lâmina 4"));
}

#[test]
fn im_b10_soltar_sobre_um_quadro_continua_igual() {
    let project = album(2);
    let photo = project.photos[5].id;
    let mut today = editor_at(project.clone(), 0);
    assert!(today.drop_photo(0, photo));
    let mut now = editor_at(project, 0);
    assert!(now.drop_photos(DropPlace::Frame(0), &[photo]));
    assert_eq!(
        now.session().unwrap().project(),
        today.session().unwrap().project()
    );
}

#[test]
fn im_b11_a_lamina_nova_respeita_a_montagem() {
    let mut project = album(3);
    project.mounting = Mounting {
        crop: CropLevel::Whole,
        ..Mounting::default()
    };
    let six = photos(&project, 6);
    let mut editor = editor_at(project, 0);
    assert!(editor.drop_photos(DropPlace::NewSpread(3), &six));
    let after = editor.session().unwrap().project();
    let trim = after.spec.trim_rect();
    for frame in &after.spreads[3].frames {
        let FrameContent::Photo { photo, .. } = &frame.content else {
            continue;
        };
        let r = frame.rect;
        let frame_aspect = r.width() * trim.width.get() / (r.height() * trim.height.get());
        let photo_aspect = after.photo(*photo).unwrap().aspect_ratio();
        let visible = (frame_aspect / photo_aspect).min(photo_aspect / frame_aspect);
        assert!(1.0 - visible < 1e-6, "recorte de {}", 1.0 - visible);
    }
}

#[test]
fn im_b14_fotos_de_fora_na_area_de_lamina_nova() {
    let project = album(6);
    let mut editor = editor_at(project, 0);
    let steps = editor.session().unwrap().history_list().steps.len();
    let entries: Vec<CatalogEntry> = (0..4)
        .map(|i| CatalogEntry {
            path: PathBuf::from(format!("/fora/n{i}.jpg")),
            fingerprint: format!("fp-fora-{i}"),
            file_size: 1,
            modified: 0,
            width: 3000,
            height: 2000,
            exif_orientation: 1,
            taken_at: None,
            camera: None,
            thumbnail: PathBuf::new(),
        })
        .collect();
    let report = ImportReport {
        entries,
        ..ImportReport::default()
    };
    let summary = editor.add_photos_as_spread(&report, 6).unwrap();
    assert_eq!(summary.added, 4);
    let after = editor.session().unwrap().project();
    assert_eq!(after.photos.len(), 24 + 4);
    assert_eq!(after.spreads.len(), 7);
    let new: Vec<PhotoId> = spread_photos(after, 6);
    let paths: Vec<String> = new
        .iter()
        .map(|id| after.photo(*id).unwrap().path.display().to_string())
        .collect();
    assert_eq!(paths.len(), 4);
    assert!(paths.iter().all(|p| p.contains("fora")), "{paths:?}");
    assert_eq!(
        editor.session().unwrap().history_list().steps.len(),
        steps + 1,
        "numa edição só"
    );
}

#[test]
fn arrastar_uma_foto_da_selecao_leva_a_selecao_inteira() {
    let project = album(2);
    let (a, b, c) = (
        project.photos[0].id,
        project.photos[1].id,
        project.photos[2].id,
    );
    let mut editor = editor_at(project, 0);
    let shown: Vec<PhotoId> = editor
        .session()
        .unwrap()
        .project()
        .photos
        .iter()
        .map(|p| p.id)
        .collect();
    editor.click_photo(&shown, 0, false, false);
    editor.click_photo(&shown, 1, true, false);
    assert_eq!(editor.drag_photos(a), [a, b]);
    assert_eq!(editor.drag_photos(c), [c], "fora da seleção, só ela");
}
