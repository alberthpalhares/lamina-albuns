//! Cenários da spec `fotos-no-album` (IM), parte de importar: fotos entrando num álbum aberto, em
//! ordem de captura, sem repetir, com o resumo e desfazendo numa edição só (IM-B1 a IM-B6, IM-B12,
//! IM-B13 e IM-B15). Soltar fotos numa lâmina fica para a parte 2.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{CaptureTime, Project};
use lamina_media::{CatalogEntry, ImportReport};

use super::common::{Base, isolated};
use super::tl_common::{H, built, editor_at, spec};
use crate::importing::{ALREADY_IN, ImportSummary, add_imported, import_photos, photos_in};
use crate::{Command, Editor, Filter, create_album, photo_list};

/// Uma foto do catálogo tirada às `hh:mm` de 12/09/2026.
fn shot(name: &str, at: &str) -> CatalogEntry {
    CatalogEntry {
        path: PathBuf::from(format!("/fotos/{name}.jpg")),
        fingerprint: format!("fp-{name}"),
        file_size: 1,
        modified: 0,
        width: 3000,
        height: 2000,
        exif_orientation: 1,
        taken_at: CaptureTime::from_exif(&format!("2026:09:12 {at}:00")),
        camera: None,
        thumbnail: PathBuf::new(),
    }
}

fn report(entries: Vec<CatalogEntry>) -> ImportReport {
    ImportReport {
        entries,
        ..ImportReport::default()
    }
}

/// Um álbum com as fotos do catálogo `entries`, sem lâminas.
fn album_with(entries: &[CatalogEntry]) -> Project {
    let mut project = built(spec(30.0, 30.0), &[H], 1);
    project.photos.clear();
    project.spreads.clear();
    add_imported(&mut project, &report(entries.to_vec()));
    project
}

fn times(project: &Project) -> Vec<String> {
    project
        .photos
        .iter()
        .map(|p| {
            p.taken_at
                .map(|t| t.to_string()[11..16].to_owned())
                .unwrap_or_default()
        })
        .collect()
}

#[test]
fn im_b1_importar_acrescenta_em_ordem_de_captura() {
    let old: Vec<CatalogEntry> = (0..10)
        .map(|i| shot(&format!("a{i}"), &format!("14:{:02}", i * 6)))
        .collect();
    let mut editor = Editor::new();
    editor.adopt(album_with(&old));
    let new = report(vec![
        shot("b1", "13:00"),
        shot("b2", "14:30"),
        shot("b3", "16:00"),
    ]);
    let summary = editor.add_photos(&new).unwrap();
    assert_eq!(summary.added, 3);
    let project = editor.session().unwrap().project();
    assert_eq!(project.photos.len(), 13);
    let t = times(project);
    let mut sorted = t.clone();
    sorted.sort();
    assert_eq!(t, sorted, "em ordem de captura");
    assert_eq!((t[0].as_str(), t[12].as_str()), ("13:00", "16:00"));
    let history = editor.session().unwrap().history_list();
    assert!(
        history
            .steps
            .last()
            .unwrap()
            .label
            .starts_with("Importar 3 fotos"),
        "{:?}",
        history.steps.last()
    );
}

#[test]
fn im_b2_repetidas_nao_entram() {
    let mut editor = Editor::new();
    editor.adopt(album_with(&[shot("a", "14:00")]));
    // A mesma foto copiada para outra pasta tem a mesma impressão digital.
    let mut copy = shot("a", "14:00");
    copy.path = PathBuf::from("/outra/pasta/a.jpg");
    let summary = editor
        .add_photos(&report(vec![copy, shot("b", "15:00")]))
        .unwrap();
    assert_eq!((summary.added, summary.repeated), (1, 1));
    assert_eq!(editor.session().unwrap().project().photos.len(), 2);
    assert_eq!(summary.text(), "1 foto importada · 1 já estava no álbum");
}

#[test]
fn im_b3_pasta_com_subpastas_e_arquivos_que_nao_sao_foto() {
    let base = Base::with_photos(6, (60, 40));
    let folder = base.path("fotos");
    std::fs::create_dir_all(folder.join("sub")).unwrap();
    for name in ["f04.jpg", "f05.jpg"] {
        std::fs::rename(folder.join(name), folder.join("sub").join(name)).unwrap();
    }
    std::fs::write(folder.join("leia-me.txt"), "não é foto").unwrap();
    let report = import_photos(&[folder], &base.cache(), &|_, _| {}).unwrap();
    assert_eq!(report.entries.len(), 6);
    assert_eq!(report.not_photos, 1);
    let mut project = album_with(&[]);
    let summary = add_imported(&mut project, &report);
    assert_eq!(summary.text(), "6 fotos importadas · 1 arquivo não é foto");
}

#[test]
fn im_b4_desfazer_a_importacao() {
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H, H, H], 1), 0);
    let before = photo_list(editor.session().unwrap().project(), Filter::All);
    let five: Vec<CatalogEntry> = (0..5).map(|i| shot(&format!("n{i}"), "10:00")).collect();
    editor.add_photos(&report(five)).unwrap();
    assert_eq!(editor.session().unwrap().project().photos.len(), 8);
    assert!(editor.run(Command::Undo));
    let project = editor.session().unwrap().project();
    assert_eq!(project.photos.len(), 3);
    assert_eq!(photo_list(project, Filter::All), before);
}

#[test]
fn im_b5_importar_uma_lista_usa_o_mesmo_catalogo_da_pasta() {
    let base = Base::with_photos(5, (60, 40));
    // Um álbum feito antes da pasta deixa as fotos no catálogo.
    create_album(&base.new_album(Some(1)), &base.cache(), &|_| {}).unwrap();
    let files: Vec<PathBuf> = (0..5)
        .map(|i| base.path("fotos").join(format!("f{i:02}.jpg")))
        .collect();
    let report = import_photos(&files, &base.cache(), &|_, _| {}).unwrap();
    assert_eq!((report.reused, report.processed), (5, 0));
}

#[test]
fn im_b6_album_sem_fotos_recebe_as_primeiras() {
    let mut editor = Editor::new();
    editor.adopt(album_with(&[]));
    let thirty: Vec<CatalogEntry> = (0..30)
        .map(|i| shot(&format!("p{i:02}"), &format!("10:{i:02}")))
        .collect();
    editor.add_photos(&report(thirty)).unwrap();
    let project = editor.session().unwrap().project();
    assert_eq!((project.photos.len(), project.spreads.len()), (30, 0));
    let unused = photo_list(project, Filter::Unused);
    assert_eq!(unused.len(), 30, "todas como não usadas");
    assert!(editor.available(Command::ImportPhotos));
}

#[test]
fn im_b12_sem_album_as_fotos_soltas_viram_o_novo_projeto() {
    let base = Base::with_photos(14, (60, 40));
    let files: Vec<PathBuf> = (0..12)
        .map(|i| base.path("fotos").join(format!("f{i:02}.jpg")))
        .collect();
    let mut new = base.new_album(Some(3));
    new.folder = PathBuf::new();
    new.files = files;
    let (built, _) = create_album(&new, &base.cache(), &|_| {}).unwrap();
    assert_eq!(
        photos_in(&new.files),
        12,
        "a etapa 3 mostra o número antes de importar"
    );
    assert_eq!(
        built.project.photos.len(),
        12,
        "só as soltas, não a pasta inteira"
    );
    // Sem álbum aberto, importar não está disponível: a janela abre o Novo projeto.
    assert!(!Editor::new().available(Command::ImportPhotos));
    assert_eq!(Command::ImportPhotos.name(), "arquivo.importar");
    assert_eq!(
        Command::from_shortcut(crate::Shortcut::ctrl('i')),
        Some(Command::ImportPhotos)
    );
}

#[test]
fn im_b13_todas_repetidas_nao_mexem_no_historico() {
    let mut editor = Editor::new();
    editor.adopt(album_with(&[shot("a", "10:00"), shot("b", "11:00")]));
    let steps = editor.session().unwrap().history_list().steps.len();
    let summary = editor
        .add_photos(&report(vec![shot("a", "10:00"), shot("b", "11:00")]))
        .unwrap();
    assert_eq!(summary.added, 0);
    assert_eq!(summary.text(), ALREADY_IN);
    assert_eq!(editor.session().unwrap().history_list().steps.len(), steps);
    assert_eq!(
        ImportSummary::default().text(),
        crate::importing::NO_PHOTOS_DROPPED
    );
}

#[test]
fn im_b15_importar_rapido() {
    let base = Base::with_photos(200, (60, 40));
    let folder = base.path("fotos");
    let cache = base.cache();
    // A primeira vez põe as 200 no catálogo.
    import_photos(std::slice::from_ref(&folder), &cache, &|_, _| {}).unwrap();
    let files: Vec<PathBuf> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let started = Instant::now();
        let report = isolated(|| import_photos(&files, &cache, &|_, _| {}).unwrap());
        let mut project = album_with(&[]);
        add_imported(&mut project, &report);
        best = best.min(started.elapsed());
        // Todas vêm do catálogo; fotos de teste iguais (mesma impressão digital) entram uma vez.
        assert_eq!((report.entries.len(), report.reused), (200, 200));
        let unique: std::collections::HashSet<&str> = report
            .entries
            .iter()
            .map(|e| e.fingerprint.as_str())
            .collect();
        assert_eq!(project.photos.len(), unique.len());
        if best < budget(300) {
            break;
        }
    }
    assert!(
        best < budget(300),
        "importar 200 fotos do catálogo levou {best:?}"
    );
}

#[test]
fn resumo_conta_formato_e_leitura() {
    let summary = ImportSummary {
        added: 24,
        repeated: 3,
        unsupported: 1,
        not_photos: 2,
        unreadable: vec![("x.jpg".into(), "sem permissão de acesso".into())],
        ..ImportSummary::default()
    };
    assert_eq!(
        summary.text(),
        "24 fotos importadas · 3 já estavam no álbum · 1 em formato não suportado · \
         2 arquivos não são fotos · 1 foto não pôde ser lida: x.jpg (sem permissão de acesso)"
    );
}
