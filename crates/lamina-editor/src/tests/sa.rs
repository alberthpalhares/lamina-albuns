//! Cenários da spec `sair-pelo-menu` (SA): o comando Sair no registro e a pergunta de salvar.

use super::tl_common::{editor_at, three_spreads};
use crate::{Command, Editor, Leave, Menu, Shortcut};

#[test]
fn sa_b1_sair_esta_no_registro_com_ctrl_q() {
    let quit = Command::from_name("arquivo.sair").unwrap();
    assert_eq!(quit, Command::Quit);
    assert_eq!(quit.label(), "Sair");
    assert_eq!(quit.menu(), Menu::File);
    assert_eq!(quit.shortcut(), Some(Shortcut::ctrl('q')));
    assert_eq!(
        Command::from_shortcut(Shortcut::ctrl('q')),
        Some(Command::Quit)
    );
    assert!(Editor::new().available(quit), "sem álbum aberto");
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.available(quit), "com álbum");
    assert!(editor.run(Command::Organize));
    assert!(editor.available(quit), "no modo organizar");
}

#[test]
fn sa_b2_sair_com_alteracoes_pergunta_antes() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = editor_at(three_spreads(), 1);
    editor
        .session_mut()
        .unwrap()
        .save_as(&dir.path().join("album.lamina"))
        .unwrap();
    assert!(!editor.needs_confirmation(), "sem alterações, fecha direto");
    assert!(editor.run(Command::NextLayout));
    assert!(editor.needs_confirmation(), "com alterações, pergunta");
    assert!(
        !editor.leave(Leave::Cancel, None).unwrap(),
        "Cancelar mantém o Lamina aberto"
    );
    assert!(editor.needs_confirmation(), "e as alterações continuam");
}
