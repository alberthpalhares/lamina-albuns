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
