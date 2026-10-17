//! Registro de comandos (JP-B12).

use std::collections::HashSet;

use super::common::Base;
use crate::{Command, Editor, Key, Shortcut};

#[test]
fn jp_b12_registro_de_comandos() {
    let names: HashSet<_> = Command::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(names.len(), Command::ALL.len(), "nomes repetidos");
    let shortcuts: Vec<Shortcut> = Command::ALL.iter().filter_map(|c| c.shortcut()).collect();
    let unique: HashSet<_> = shortcuts.iter().collect();
    assert_eq!(unique.len(), shortcuts.len(), "atalhos repetidos");
    // As setas e Ctrl+Z/Ctrl+Y ficaram reservadas no JP e passaram para o TL (TL-B12).

    let mut editor = Editor::new();
    for command in [Command::NewAlbum, Command::Open, Command::About] {
        assert!(editor.available(command), "{}", command.name());
    }
    for command in [
        Command::Save,
        Command::SaveAs,
        Command::NextSpread,
        Command::LastSpread,
    ] {
        assert!(!editor.available(command), "{} sem projeto", command.name());
    }

    let base = Base::with_photos(8, (600, 400));
    editor.adopt(base.project(2));
    for command in [
        Command::Save,
        Command::SaveAs,
        Command::NextSpread,
        Command::LastSpread,
    ] {
        assert!(editor.available(command), "{} com projeto", command.name());
    }
    assert!(!editor.available(Command::PreviousSpread), "já na primeira");
}

#[test]
fn atalho_leva_ao_comando() {
    let save_as = Shortcut {
        ctrl: true,
        shift: true,
        alt: false,
        key: Key::Char('s'),
    };
    assert_eq!(Command::from_shortcut(save_as), Some(Command::SaveAs));
    assert_eq!(save_as.text(), "Ctrl+Shift+S");
    let page_down = Command::NextSpread.shortcut().unwrap();
    assert_eq!(page_down.text(), "Page Down");
    assert_eq!(Command::from_name("ver.guias"), Some(Command::ToggleGuides));
    assert_eq!(Key::from_name("PageDown"), Some(Key::PageDown));
    assert_eq!(Key::from_name("S"), Some(Key::Char('s')));
    assert_eq!(Key::from_name("\u{1b}"), None);
    assert_eq!(Key::from_name("ab"), None);
    let mut editor = Editor::new();
    assert!(
        !editor.run(Command::ToggleGuides),
        "sem projeto, sem lâmina"
    );
    editor.adopt(Base::with_photos(4, (600, 400)).project(1));
    assert!(editor.guides());
    assert!(editor.run(Command::ToggleGuides));
    assert!(!editor.guides());
}
