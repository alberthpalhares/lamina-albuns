//! Registro de comandos (JP-B12).

use std::collections::HashSet;

use super::common::Base;
use crate::commands::Mode;
use crate::{Command, Editor, Key, Shortcut};

#[test]
fn jp_b12_registro_de_comandos() {
    let names: HashSet<_> = Command::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(names.len(), Command::ALL.len(), "nomes repetidos");
    // Sem repetir dentro do mesmo modo (spec EG, regra 2: as setas movem no modo organizar), nos
    // dois modos; nenhum alternativo é atalho principal (spec AT, regra 10).
    for mode in [Mode::Normal, Mode::Organize] {
        let shortcuts: Vec<Shortcut> = Command::ALL
            .iter()
            .filter(|c| c.in_mode(mode))
            .filter_map(|c| c.shortcut())
            .collect();
        let unique: HashSet<_> = shortcuts.iter().collect();
        assert_eq!(
            unique.len(),
            shortcuts.len(),
            "atalhos repetidos em {mode:?}"
        );
    }
    for (alias, _) in crate::shortcuts::ALIASES {
        assert!(Command::ALL.iter().all(|c| c.shortcut() != Some(alias)));
    }
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

#[test]
fn comandos_da_janela_nova_ficam_registrados_e_inertes() {
    // Rodada 0b: os comandos entram na tabela antes das specs, mas nenhum roda ainda.
    let new = [
        (Command::ToggleLayoutBand, "ver.faixa-layouts"),
        (Command::Loupe, "organizar.lupa"),
        (Command::ActualSize, "organizar.tamanho-real"),
        (Command::CalibrateSize, "organizar.calibrar"),
        (Command::BuildAlbum, "lamina.montar-album"),
        (Command::SelectAllPhotos, "editar.selecionar-fotos"),
    ];
    let mut editor = Editor::new();
    for (command, name) in new {
        assert_eq!(Command::from_name(name), Some(command));
        assert!(!editor.available(command), "{name} sem álbum");
    }
    editor.adopt(Base::with_photos(8, (600, 400)).project(2));
    for (command, name) in new {
        assert!(!editor.available(command), "{name} no modo normal");
        assert!(!editor.run(command), "{name} não roda");
    }
    assert!(editor.run(Command::Organize));
    for (command, name) in new {
        assert!(!editor.available(command), "{name} no modo organizar");
    }
}

#[test]
fn l_z_ctrl_a_ctrl_barra_e_f1_levam_aos_comandos_novos() {
    let key = |c| Shortcut::key(Key::Char(c));
    let in_mode = Command::from_shortcut_in;
    // Ctrl+A: as fotos mostradas fora do modo; no modo organizar, os quadros (spec EG).
    assert_eq!(
        in_mode(Shortcut::ctrl('a'), Mode::Normal),
        Some(Command::SelectAllPhotos)
    );
    assert_eq!(
        in_mode(Shortcut::ctrl('a'), Mode::Organize),
        Some(Command::SelectAllFrames)
    );
    assert_eq!(
        in_mode(key('l'), Mode::Normal),
        Some(Command::ToggleLayoutBand)
    );
    assert_eq!(in_mode(key('l'), Mode::Organize), None, "L só fora do modo");
    assert_eq!(in_mode(key('z'), Mode::Organize), Some(Command::Loupe));
    assert_eq!(in_mode(key('z'), Mode::Normal), None, "Z só no modo");
    assert_eq!(
        Command::from_shortcut(Shortcut::ctrl('/')),
        Some(Command::ShowShortcuts)
    );
    assert_eq!(Shortcut::ctrl('/').text(), "Ctrl+/");
    // O F1 é o alternativo do Ctrl+/ (spec AT, regra 3).
    assert_eq!(Key::from_name("F1"), Some(Key::F1));
    assert_eq!(Shortcut::key(Key::F1).text(), "F1");
    assert_eq!(
        Command::from_shortcut(Shortcut::key(Key::F1)),
        Some(Command::ShowShortcuts)
    );
    // Nunca Ctrl+número: o dono usa o ZoomIt.
    for c in Command::ALL {
        if let Some(s) = c.shortcut() {
            let digit = matches!(s.key, Key::Char(d) if d.is_ascii_digit());
            assert!(!(s.ctrl && digit), "{c:?} usa Ctrl+número");
        }
    }
}

#[test]
fn todo_comando_esta_numa_parte_da_tabela() {
    for c in Command::ALL {
        assert!(!c.name().is_empty(), "{c:?} sem nome");
        assert!(!c.label().is_empty(), "{c:?} sem rótulo");
        assert_eq!(Command::from_name(c.name()), Some(c));
    }
}
