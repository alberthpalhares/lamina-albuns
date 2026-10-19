//! Cenários da spec `primeiro-contato` (PC) que rodam sem janela: os comandos da boas-vindas e a
//! cópia do álbum de exemplo. O PC-B5 (o exemplo do repositório) e o PC-B7 (as notas) moram no
//! xtask; o PC-B8 (`--verificar exemplo`), nos testes do `lamina-app`.

use std::fs;
use std::path::{Path, PathBuf};

use super::common::Base;
use super::tl_common::{editor_at, three_spreads};
use crate::{
    Command, EXAMPLE_FILE, Editor, EditorError, Human, Session, create_album, prepare_example,
};

/// Um exemplo pequeno como o do programa: `exemplo.lamina` e `fotos/` na mesma pasta.
fn example_source(base: &Base) -> PathBuf {
    let src = base.path("programa/exemplo");
    fs::create_dir_all(src.join("fotos")).unwrap();
    for entry in fs::read_dir(base.path("fotos")).unwrap() {
        let from = entry.unwrap().path();
        fs::copy(&from, src.join("fotos").join(from.file_name().unwrap())).unwrap();
    }
    let mut request = base.new_album(Some(3));
    request.folder = src.join("fotos");
    let (built, _) = create_album(&request, &base.cache(), &|_| {}).unwrap();
    Session::new_unsaved(built.project)
        .save_as(&src.join(EXAMPLE_FILE))
        .unwrap();
    src
}

fn opened(file: &Path) -> Editor {
    let mut editor = Editor::new();
    editor.open(file).unwrap();
    editor
}

#[test]
fn pc_b1_o_que_a_boas_vindas_oferece() {
    let empty = Editor::new();
    for command in [Command::OpenExample, Command::NewAlbum, Command::Open] {
        assert!(empty.available(command), "{command:?}");
    }
    assert_eq!(Command::OpenExample.name(), "arquivo.exemplo");
    let open = editor_at(three_spreads(), 0);
    assert!(open.available(Command::OpenExample));
}

#[test]
fn pc_b2_a_primeira_vez_copia_e_abre() {
    let base = Base::with_photos(6, (600, 400));
    let src = example_source(&base);
    let target = base.path("config/Álbum de exemplo");
    let file = prepare_example(&src, &target).unwrap();
    assert_eq!(file, target.join("Álbum de exemplo.lamina"));
    assert_eq!(fs::read_dir(target.join("fotos")).unwrap().count(), 6);
    let editor = opened(&file);
    let session = editor.session().unwrap();
    assert!(session.missing().is_empty(), "{:?}", session.missing());
    assert_eq!(session.spread_count(), 3);
}

#[test]
fn pc_b3_a_segunda_vez_abre_a_mesma_copia() {
    let base = Base::with_photos(6, (600, 400));
    let src = example_source(&base);
    let target = base.path("config/Álbum de exemplo");
    let file = prepare_example(&src, &target).unwrap();
    let mut editor = opened(&file);
    assert!(editor.run(Command::DeleteSpread));
    editor.session_mut().unwrap().save().unwrap();
    let again = prepare_example(&src, &target).unwrap();
    assert_eq!(again, file);
    assert_eq!(opened(&again).session().unwrap().spread_count(), 2);
}

#[test]
fn pc_b4_uma_copia_incompleta_e_completada() {
    let base = Base::with_photos(6, (600, 400));
    let src = example_source(&base);
    let target = base.path("config/Álbum de exemplo");
    let file = prepare_example(&src, &target).unwrap();
    fs::write(&file, fs::read(&file).unwrap()).unwrap();
    let before = fs::read(&file).unwrap();
    let photo = target.join("fotos").join("f03.jpg");
    fs::remove_file(&photo).unwrap();
    prepare_example(&src, &target).unwrap();
    assert!(photo.is_file());
    assert_eq!(fs::read(&file).unwrap(), before);
}

#[test]
fn pc_b6_exemplo_ausente() {
    let base = Base::with_photos(1, (600, 400));
    let err = prepare_example(&base.path("sem-exemplo"), &base.path("destino")).unwrap_err();
    assert!(matches!(err, EditorError::NoExample(_)), "{err}");
    let m = err.human();
    assert_eq!(m.title, "O álbum de exemplo não veio com esta instalação.");
    assert!(m.what_to_do.contains("Novo álbum"), "{m:?}");
    assert!(!base.path("destino").exists());
}
