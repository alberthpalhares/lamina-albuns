//! Cenários da spec `abrir-no-mac` (MA) que vivem no editor: o que fazer com os arquivos que o
//! sistema entrega (duplo clique, "Abrir com", Dock, argumento).

use std::path::{Path, PathBuf};

use super::common::Base;
use crate::received::{NOT_AN_ALBUM, Received, choose, is_album, one_at_a_time};
use crate::{Editor, Leave};

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

#[test]
fn ma_b1_abre_o_album_recebido() {
    let got = choose(&paths(&["/fotos/Casamento.lamina"]), None, false);
    assert_eq!(
        got,
        Received::Open {
            path: PathBuf::from("/fotos/Casamento.lamina"),
            ask: false,
            notice: None,
        }
    );
}

#[test]
fn ma_b2_mais_de_um_album_abre_o_primeiro_e_avisa() {
    let got = choose(
        &paths(&["/a/Casamento.lamina", "/a/Batizado.lamina"]),
        None,
        false,
    );
    assert_eq!(
        got,
        Received::Open {
            path: PathBuf::from("/a/Casamento.lamina"),
            ask: false,
            notice: Some("O Lamina abre um álbum por vez: abriu “Casamento”.".to_owned()),
        }
    );
    assert_eq!(
        one_at_a_time("Casamento"),
        "O Lamina abre um álbum por vez: abriu “Casamento”."
    );
}

#[test]
fn ma_b3_nada_que_seja_album_so_avisa() {
    let open = Path::new("/a/Atual.lamina");
    let got = choose(&paths(&["/fotos/IMG_0001.jpg"]), Some(open), true);
    assert_eq!(
        got,
        Received::Ignore {
            notice: NOT_AN_ALBUM.to_owned()
        }
    );
    assert_eq!(NOT_AN_ALBUM, "O Lamina só abre álbuns (arquivos .lamina).");
    assert!(matches!(choose(&[], None, false), Received::Ignore { .. }));
}

#[test]
fn ma_b4_extensao_em_maiusculas_entre_outros_arquivos() {
    let got = choose(&paths(&["/a/foto.jpg", "/a/FESTA.LAMINA"]), None, false);
    assert_eq!(
        got,
        Received::Open {
            path: PathBuf::from("/a/FESTA.LAMINA"),
            ask: false,
            notice: None,
        }
    );
    assert!(is_album(Path::new("x.Lamina")));
    assert!(!is_album(Path::new("lamina")), "sem extensão");
    assert!(!is_album(Path::new("x.lamina.zip")));
}

#[test]
fn ma_b5_o_album_que_ja_esta_aberto() {
    let base = Base::with_photos(8, (600, 400));
    let file = base.save(base.project(2), "album/casamento.lamina");
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    assert_eq!(
        editor.receive(std::slice::from_ref(&file)),
        Received::AlreadyOpen
    );
    // Por outro caminho até o mesmo arquivo (com `..`), também.
    let other = base.path("album/../album/casamento.lamina");
    assert_eq!(editor.receive(&[other]), Received::AlreadyOpen);
    // Com alterações, continua o mesmo: nada é relido nem perguntado.
    assert_eq!(
        choose(std::slice::from_ref(&file), Some(&file), true),
        Received::AlreadyOpen
    );
}

/// Editor com um álbum novo, nunca salvo (alterações), e o álbum `other` recebido: a decisão
/// pede a pergunta e o recebido fica à espera.
fn waiting(base: &Base, other: &Path) -> Editor {
    let mut editor = Editor::new();
    editor.adopt(base.project(2));
    assert!(editor.needs_confirmation(), "álbum novo, não salvo");
    assert_eq!(
        editor.receive(&[other.to_path_buf()]),
        Received::Open {
            path: other.to_path_buf(),
            ask: true,
            notice: None,
        }
    );
    editor
}

/// O caminho inteiro da janela, sem janela: receber com alterações → "Salvar as alterações?" →
/// Cancelar, Não salvar ou Salvar (`files::answer` chama o `leave` e, se seguiu, o
/// `opened::proceed`, que pede o `take_received` e abre).
#[test]
fn ma_b6_com_alteracoes_pergunta_antes() {
    let base = Base::with_photos(8, (600, 400));
    let other = base.save(base.project(2), "album/batizado.lamina");
    // Cancelar: o atual fica, com as alterações, e o recebido é esquecido.
    let mut editor = waiting(&base, &other);
    assert!(!editor.leave(Leave::Cancel, None).unwrap());
    assert_eq!(editor.take_received(), None);
    assert!(editor.session().is_some_and(|s| s.file().is_none()));
    assert!(editor.needs_confirmation());
    // Não salvar: o recebido sai da espera uma vez só e abre.
    let mut editor = waiting(&base, &other);
    assert!(editor.leave(Leave::DontSave, None).unwrap());
    let path = editor.take_received().unwrap();
    assert_eq!(editor.take_received(), None, "uma vez só");
    editor.open(&path).unwrap();
    assert_eq!(
        editor.session().and_then(|s| s.file()),
        Some(other.as_path())
    );
    // Salvar: grava o atual onde o fotógrafo escolher e abre o recebido.
    let mut editor = waiting(&base, &other);
    let saved = base.path("album/atual.lamina");
    assert!(editor.leave(Leave::Save, Some(&saved)).unwrap());
    assert!(saved.exists());
    let path = editor.take_received().unwrap();
    editor.open(&path).unwrap();
    assert!(!editor.needs_confirmation());
    // Sem alterações, abre direto e nada fica à espera.
    assert!(matches!(
        editor.receive(&[base.path("album/outro.lamina")]),
        Received::Open { ask: false, .. }
    ));
    assert_eq!(editor.take_received(), None);
}
