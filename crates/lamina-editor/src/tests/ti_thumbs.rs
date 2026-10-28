//! Cenários da spec `tela-inicial` (TI) sobre a miniatura do cartão: a primeira lâmina, o álbum
//! sem lâminas (spec BR) e a miniatura relida do disco depois de salvar (correção de 2026-10-22).

use super::common::Base;
use super::ti::{NOW, entry};
use crate::Editor;
use crate::recents::{RecentsStore, cards, read_thumbnail, thumbnail_image, write_thumbnail};

#[test]
fn ti_b15_a_miniatura_e_a_primeira_lamina_mesmo_parado_em_outra() {
    let base = Base::with_photos(8, (600, 400));
    let file = base.save(base.project(3), "album/casamento.lamina");
    let mut editor = Editor::new();
    editor.set_recents(RecentsStore::load(&base.path("config/recentes.json")));
    editor.open(&file).unwrap();
    assert!(editor.go_to_spread(2));
    editor.save_to(None).unwrap();
    let (project, _) = editor.thumbnail_job().unwrap();
    let thumb = thumbnail_image(&project, &base.cache()).unwrap().unwrap();
    // Só com a primeira lâmina, a miniatura é a mesma; só com a terceira, é outra.
    let mut first = project.clone();
    first.spreads.truncate(1);
    let mut third = project.clone();
    third.spreads.drain(..2);
    let cache = base.cache();
    assert_eq!(thumb, thumbnail_image(&first, &cache).unwrap().unwrap());
    assert_ne!(thumb, thumbnail_image(&third, &cache).unwrap().unwrap());
}

#[test]
fn ti_b16_album_sem_laminas_mostra_o_desenho_do_album_em_branco() {
    let base = Base::with_photos(4, (600, 400));
    let mut project = base.project(2);
    let target = base.path("config/recentes/a.png");
    write_thumbnail(&project, &base.cache(), &target).unwrap();
    assert!(target.exists());
    // Todas as lâminas apagadas (ou o álbum que a BR cria em branco): a miniatura velha sai.
    project.spreads.clear();
    assert_eq!(thumbnail_image(&project, &base.cache()), Ok(None));
    write_thumbnail(&project, &base.cache(), &target).unwrap();
    assert!(!target.exists());
    // E o cartão sabe que é um álbum em branco, para desenhar em vez do retângulo escuro.
    let mut blank = entry(&base.path("b.lamina"), "Ana");
    blank.spreads = 0;
    let full = entry(&base.path("c.lamina"), "Bia");
    let shown: Vec<bool> = cards(&[blank, full], &[], NOW)
        .iter()
        .map(|c| c.blank)
        .collect();
    assert_eq!(shown, [true, false]);
}

#[test]
fn ti_b17_a_miniatura_e_relida_do_disco_depois_de_salvar() {
    let base = Base::with_photos(8, (600, 400));
    let project = base.project(2);
    let target = base.path("config/recentes/a.png");
    write_thumbnail(&project, &base.cache(), &target).unwrap();
    let before = read_thumbnail(&target).unwrap();
    assert_eq!(before.width.max(before.height), 320);
    // O fotógrafo troca a primeira lâmina e salva: o cartão mostra a nova, não a guardada.
    let mut changed = project.clone();
    changed.spreads.swap(0, 1);
    write_thumbnail(&changed, &base.cache(), &target).unwrap();
    assert_ne!(before, read_thumbnail(&target).unwrap());
    assert!(read_thumbnail(&base.path("config/recentes/nenhuma.png")).is_none());
}

#[test]
fn ti_b18_abrir_um_album_ja_prepara_a_miniatura() {
    let base = Base::with_photos(4, (600, 400));
    let file = base.save(base.project(1), "album/antigo.lamina");
    let mut editor = Editor::new();
    editor.set_recents(RecentsStore::load(&base.path("config/recentes.json")));
    // Aberto e fechado sem salvar (um álbum de antes da TI, ou o de exemplo).
    editor.open(&file).unwrap();
    let (project, target) = editor.thumbnail_job().unwrap();
    assert_eq!(Some(target.clone()), editor.recents().thumb_path(&file));
    write_thumbnail(&project, &base.cache(), &target).unwrap();
    assert!(editor.close_album());
    assert!(read_thumbnail(&target).is_some());
}

#[test]
fn ti_b19_foto_ausente_na_primeira_lamina() {
    use crate::recents::MISSING_GRAY;
    let base = Base::with_photos(8, (600, 400));
    let project = base.project(2);
    // Uma foto da primeira lâmina sem o original e sem a cópia reduzida.
    let first = crate::edits::spread_photos(&project, 0);
    assert!(first.len() >= 2, "a lâmina 1 tem várias fotos");
    let gone = project.photo(first[0]).unwrap().clone();
    std::fs::remove_file(&gone.path).unwrap();
    let mut cache_less = project.clone();
    for p in &mut cache_less.photos {
        if p.id == gone.id {
            p.fingerprint = Some("sem-copia".to_owned());
        }
    }
    let thumb = thumbnail_image(&cache_less, &base.cache())
        .unwrap()
        .expect("a miniatura existe mesmo com a foto ausente");
    let gray = thumb.pixels().filter(|p| p.0 == MISSING_GRAY).count();
    assert!(
        gray > 100,
        "o quadro da foto que falta é cinza ({gray} pixels)"
    );
}

#[test]
fn ti_b20_a_miniatura_lida_fica_guardada() {
    use crate::recents::ThumbCache;
    let base = Base::with_photos(4, (600, 400));
    let target = base.path("config/recentes/a.png");
    write_thumbnail(&base.project(1), &base.cache(), &target).unwrap();
    let mut seen = ThumbCache::default();
    assert!(seen.get(&target).is_some());
    assert!(seen.get(&target).is_some());
    assert_eq!(seen.reads, 1, "sem mudar no disco, não lê de novo");
    // Regravado (com outra data), é lido de novo.
    std::thread::sleep(std::time::Duration::from_millis(20));
    write_thumbnail(&base.project(2), &base.cache(), &target).unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&target)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(seen.get(&target).is_some());
    assert_eq!(seen.reads, 2);
}
