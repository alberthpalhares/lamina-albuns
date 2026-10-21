//! Cenários da spec `travamento-com-muitas-fotos` (TR) no editor: importar 1.000 fotos (B6), o
//! álbum sem lâminas (B10: desenhar, Enter, Shift+Enter, desfazer) e mais fotos do que cabem numa
//! lâmina nova (B11).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use image::{Rgb, RgbImage};
use lamina_core::testing::{HEAVY_TRIES, budget};
use lamina_core::{CaptureTime, PhotoId, Project};
use lamina_media::{CatalogEntry, ImportReport};
use rayon::prelude::*;

use super::tl_common::spec;
use crate::importing::import_photos;
use crate::motion::{photo_rows, strip_rows, timeline_rows};
use crate::{
    Area, Cache, Command, DropPlace, Editor, Filter, Part, ScreenPlan, load_thumb, photo_list,
    prepare_images, spread_view,
};

/// `count` fotos 48 × 32 diferentes entre si: os bits do número desenhados em blocos.
fn distinct_photos(dir: &Path, count: u32) -> PathBuf {
    let folder = dir.join("fotos");
    std::fs::create_dir_all(&folder).unwrap();
    (0..count).into_par_iter().for_each(|i| {
        let img = RgbImage::from_fn(48, 32, |x, y| {
            let bit = x / 4 + 12 * (y / 16);
            if (i >> bit) & 1 == 1 {
                Rgb([230, 230, 230])
            } else {
                Rgb([20, 60, 120])
            }
        });
        img.save(folder.join(format!("f{i:04}.jpg"))).unwrap();
    });
    folder
}

/// Um álbum como o de "Começar sem fotos": sem fotos e sem lâminas.
fn empty_editor() -> Editor {
    let mut editor = Editor::new();
    editor.adopt(Project::new("vazio", spec(30.0, 30.0)));
    editor
}

/// `count` fotos do catálogo, uma por minuto a partir das 10h.
fn report(count: usize) -> ImportReport {
    let entries = (0..count)
        .map(|i| CatalogEntry {
            path: PathBuf::from(format!("/fotos/p{i:04}.jpg")),
            fingerprint: format!("fp-{i}"),
            file_size: 1,
            modified: 0,
            width: 3000,
            height: 2000,
            exif_orientation: 1,
            taken_at: CaptureTime::from_exif(&format!(
                "2026:09:12 {:02}:{:02}:00",
                10 + i / 60,
                i % 60
            )),
            camera: None,
            thumbnail: PathBuf::new(),
        })
        .collect();
    ImportReport {
        entries,
        ..ImportReport::default()
    }
}

/// Meta da importação de 1.000 fotos pequenas, do zero (sem catálogo), em ms. Medido em
/// 2026-10-21: 3,6 s no Windows, quase tudo criando os 2.000 arquivos do cache; uma foto de cada
/// vez passaria de 14 s.
const IMPORT_MS: u64 = 8000;

#[test]
fn tr_b6_importar_mil_fotos_pequenas_rapido() {
    let dir = tempfile::tempdir().unwrap();
    let folder = distinct_photos(dir.path(), 1000);
    let mut best = Duration::MAX;
    for n in 0..HEAVY_TRIES {
        // Um cache novo a cada medida: todas as fotos são lidas de novo.
        let cache = Cache::new(dir.path().join(format!("cache{n}")));
        let started = Instant::now();
        let report = import_photos(std::slice::from_ref(&folder), &cache, &|_, _| {}).unwrap();
        let mut editor = empty_editor();
        editor.add_photos(&report).unwrap();
        best = best.min(started.elapsed());
        assert_eq!((report.processed, report.failures.len()), (1000, 0));
        assert_eq!(editor.session().unwrap().project().photos.len(), 1000);
        if best < budget(IMPORT_MS) {
            break;
        }
    }
    assert!(
        best < budget(IMPORT_MS),
        "importar 1.000 fotos pequenas levou {best:?}"
    );
}

/// Tudo o que a janela pergunta ao editor para desenhar a lâmina, as faixas e o menu.
fn draw_everything(editor: &mut Editor) {
    let s = editor.session().unwrap();
    let (project, index) = (s.project().clone(), s.current());
    let _ = (s.warnings_for(index), s.counter(), s.page_warning());
    let _ = (strip_rows(s), timeline_rows(&project));
    let items = photo_list(&project, Filter::All);
    let _ = photo_rows(&items, editor.selection());
    let area = Area {
        width: 1.0,
        height: 0.5,
        margin: 0.0,
    };
    let _ = spread_view(&project, index, area, &[]);
    let _ = editor.view_project();
    let _ = (editor.panel(Part::Spread), editor.details());
    let _ = editor.context_menu(Some(0));
    let _ = ScreenPlan::default().go(&project, index);
    for command in Command::ALL {
        let _ = editor.available(command);
    }
}

#[test]
fn tr_b10_album_sem_laminas_nao_entra_em_panico() {
    let mut editor = empty_editor();
    editor.add_photos(&report(1000)).unwrap();
    assert_eq!(editor.session().unwrap().project().spreads.len(), 0);
    draw_everything(&mut editor);
    // Uma lâmina nova pelo "+ Lâmina nova", o quadro selecionado e Ctrl+Z: a lâmina some e o
    // quadro selecionado fica sem lâmina.
    let first: Vec<PhotoId> = editor.session().unwrap().project().photos[..3]
        .iter()
        .map(|p| p.id)
        .collect();
    assert!(editor.drop_photos(DropPlace::NewSpread(0), &first));
    editor.select_frame(Some(0));
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.session().unwrap().project().spreads.len(), 0);
    draw_everything(&mut editor);
    // Fotos selecionadas e Enter: a seleção monta a primeira lâmina, sem pânico (spec BR, BR-B10).
    let photos: Vec<PhotoId> = editor.session().unwrap().project().photos[..5]
        .iter()
        .map(|p| p.id)
        .collect();
    editor.set_selection(photos.clone());
    draw_everything(&mut editor);
    assert!(editor.run(Command::BuildFromSelection));
    let project = editor.session().unwrap().project();
    assert_eq!(project.spreads.len(), 1);
    let mut placed = crate::edits::spread_photos(project, 0);
    let mut expected = photos.clone();
    placed.sort();
    expected.sort();
    assert_eq!(placed, expected, "as 5 fotos da seleção");
    assert!(editor.run(Command::Undo));
    // Shift+Enter num álbum sem lâminas: a lâmina nova é a 1, e o rótulo diz isso.
    editor.set_selection(photos);
    assert!(editor.run(Command::NewSpreadFromSelection));
    let history = editor.session().unwrap().history_list();
    assert_eq!(
        history.steps.last().unwrap().label,
        "Nova lâmina com a seleção · lâmina 1"
    );
    assert!(editor.run(Command::Undo));
    draw_everything(&mut editor);
    // As imagens que faltam de um álbum sem lâminas também (foto que não existe só falha).
    let dir = tempfile::tempdir().unwrap();
    let project = editor.session().unwrap().project().clone();
    assert_eq!(prepare_images(&project, 0, &Cache::new(dir.path())), 1000);
}

#[test]
fn tr_b11_mais_de_24_fotos_na_lamina_nova_entram_todas() {
    let mut editor = empty_editor();
    let steps = editor.session().unwrap().history_list().steps.len();
    let summary = editor.add_photos_as_spread(&report(30), 0).unwrap();
    let project = editor.session().unwrap().project();
    assert_eq!((project.photos.len(), project.spreads.len()), (30, 0));
    assert_eq!(summary.over_capacity, Some(24));
    assert_eq!(
        summary.text(),
        "30 fotos importadas · uma lâmina leva até 24 fotos: elas ficaram na faixa de fotos, \
         sem lâmina nova"
    );
    let history = editor.session().unwrap().history_list();
    assert_eq!(history.steps.len(), steps + 1, "numa edição só");
    assert!(
        history
            .steps
            .last()
            .unwrap()
            .label
            .starts_with("Importar 30 fotos")
    );
    // Até 24, continua virando a lâmina nova (IM-B14).
    let mut editor = empty_editor();
    editor.add_photos_as_spread(&report(24), 0).unwrap();
    assert_eq!(editor.session().unwrap().project().spreads.len(), 1);
}

#[test]
fn miniatura_vem_do_cache_em_rgb() {
    let dir = tempfile::tempdir().unwrap();
    let folder = distinct_photos(dir.path(), 2);
    let cache = Cache::new(dir.path().join("cache"));
    let report = import_photos(&[folder], &cache, &|_, _| {}).unwrap();
    let mut editor = empty_editor();
    editor.add_photos(&report).unwrap();
    let photo = editor.session().unwrap().project().photos[0].clone();
    let thumb = load_thumb(&photo, &cache).unwrap();
    // 320 px no lado maior, como sempre (a foto pequena é ampliada só na miniatura).
    assert_eq!(thumb.dimensions(), (320, 213));
    // Sem miniatura no cache: nada, sem erro.
    assert!(load_thumb(&photo, &Cache::new(dir.path().join("outro"))).is_none());
}
