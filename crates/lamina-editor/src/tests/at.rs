//! Cenários da spec `atalhos` (AT): a lista, os grupos, os alternativos e as teclas fora da tabela.
//! A busca, o Mac e as linhas fixas (AT-B6, AT-B8 e AT-B11) ficam em `at_busca.rs`; o AT-B9 lê os `.slint` e fica em
//! `crates/lamina-app/tests/atalhos.rs`.

use std::collections::HashSet;

use lamina_core::FrameContent;

use super::tl_common::{editor_at, three_spreads};
use crate::commands::Mode;
use crate::shortcuts::{
    ALIASES, CROP, GROUPS, Group, NORMAL_ALIASES, ORGANIZE, Row, group_of, main_of, search, sheet,
};
use crate::{Command, Editor, Key, Platform, Shortcut};

/// A linha de `does` na lista, com o grupo dela.
pub(super) fn find<'a>(groups: &'a [Group], does: &str) -> (&'a str, &'a Row) {
    groups
        .iter()
        .find_map(|g| g.rows.iter().find(|r| r.does == does).map(|r| (g.title, r)))
        .unwrap_or_else(|| panic!("sem a linha {does}"))
}

pub(super) fn keys(groups: &[Group], does: &str) -> Vec<String> {
    find(groups, does).1.keys.clone()
}

pub(super) fn all_rows(groups: &[Group]) -> Vec<&'static str> {
    groups
        .iter()
        .flat_map(|g| g.rows.iter().map(|r| r.does))
        .collect()
}

fn crop_zoom_of(editor: &Editor, frame: usize) -> f64 {
    let s = editor.session().unwrap();
    match &s.project().spreads[s.current()].frames[frame].content {
        FrameContent::Photo { crop, .. } => crop.zoom,
        _ => panic!("quadro sem foto"),
    }
}

#[test]
fn at_b1_tres_caminhos_abrem_a_lista() {
    for mode in [Mode::Normal, Mode::Organize] {
        for key in [Shortcut::ctrl('/'), Shortcut::key(Key::F1)] {
            assert_eq!(
                Command::from_shortcut_in(key, mode),
                Some(Command::ShowShortcuts),
                "{key:?} em {mode:?}"
            );
        }
    }
    let open = Command::ShowShortcuts;
    assert!(Editor::new().available(open), "sem álbum");
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.available(open), "com álbum");
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    assert!(editor.available(open), "no recorte");
    assert!(editor.run(Command::Organize) && editor.organizing());
    assert!(editor.available(open), "no modo organizar");
    // Durante uma exportação, a camada "Exportando…" ficaria por cima (SR, regra 2).
    editor.set_busy(true);
    assert!(!editor.available(open));
}

#[test]
fn at_b2_abrir_a_lista_nao_mexe_em_nada() {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    assert!(editor.crop_zoom(2.0) && editor.commit_crop());
    let history = editor.session().unwrap().history_list();
    let dirty = editor.session().unwrap().is_dirty();
    assert!(!editor.run(Command::ShowShortcuts));
    assert_eq!(editor.crop_mode(), Some(0), "o recorte continua aberto");
    assert!((crop_zoom_of(&editor, 0) - 2.0).abs() < 1e-9);
    assert_eq!(editor.session().unwrap().history_list(), history);
    assert_eq!(editor.session().unwrap().is_dirty(), dirty);

    // Em outra lâmina, o modo organizar com dois quadros selecionados.
    let mut editor = editor_at(three_spreads(), 2);
    assert!(editor.run(Command::Organize));
    editor.organize_click(0, false, false);
    editor.organize_click(1, true, false);
    assert_eq!(editor.selected_frames(), [0, 1]);
    let history = editor.session().unwrap().history_list();
    let dirty = editor.session().unwrap().is_dirty();
    assert!(!editor.run(Command::ShowShortcuts));
    assert!(editor.organizing(), "o modo continua");
    assert_eq!(editor.selected_frames(), [0, 1]);
    assert_eq!(editor.session().unwrap().history_list(), history);
    assert_eq!(editor.session().unwrap().is_dirty(), dirty);
}

#[test]
fn at_b3_todo_atalho_aparece_uma_vez() {
    let groups = sheet(Platform::Windows);
    let commands: Vec<Command> = groups
        .iter()
        .flat_map(|g| g.rows.iter().filter_map(|r| r.command))
        .collect();
    let with_key: Vec<Command> = Command::ALL
        .into_iter()
        .filter(|c| c.shortcut().is_some())
        .collect();
    assert_eq!(commands.len(), with_key.len());
    let unique: HashSet<Command> = commands.iter().copied().collect();
    assert_eq!(unique.len(), commands.len(), "um comando numa linha só");
    for c in with_key {
        let (_, row) = groups
            .iter()
            .find_map(|g| g.rows.iter().find(|r| r.command == Some(c)).map(|r| (g, r)))
            .unwrap();
        assert_eq!(row.does, c.label(), "o rótulo da tabela");
        assert_eq!(row.keys[0], c.shortcut().unwrap().text());
    }
    assert!(
        !commands.contains(&Command::Specification),
        "sem atalho, fora"
    );
}

#[test]
fn at_b4_grupos_por_menu_e_por_modo() {
    let groups = sheet(Platform::Windows);
    let titles: Vec<&str> = groups.iter().map(|g| g.title).collect();
    assert_eq!(titles, GROUPS);
    assert_eq!(
        titles,
        [
            "Arrastar e clicar",
            "Arquivo",
            "Editar",
            "Lâmina",
            "Layout",
            "Visualizar",
            "Ajuda",
            "No modo organizar",
            "Recorte da foto"
        ]
    );
    for (does, group, key) in [
        ("Salvar", "Arquivo", "Ctrl+S"),
        ("Próximo layout", "Layout", "↓"),
        ("Mover 1 mm para a esquerda", ORGANIZE, "←"),
        ("Selecionar todos os quadros", ORGANIZE, "Ctrl+A"),
        ("Recortar foto", CROP, "C"),
        ("Mais zoom", CROP, "+"),
        ("Endireitar +0,1°", CROP, "]"),
    ] {
        let (title, row) = find(&groups, does);
        assert_eq!((title, row.keys[0].as_str()), (group, key), "{does}");
    }
    // Dentro de cada grupo, as linhas da tabela na ordem de Command::ALL.
    for g in &groups {
        let order: Vec<usize> = g
            .rows
            .iter()
            .filter_map(|r| r.command)
            .map(|c| Command::ALL.iter().position(|a| *a == c).unwrap())
            .collect();
        assert!(order.is_sorted(), "{}", g.title);
        for r in &g.rows {
            if let Some(c) = r.command {
                assert_eq!(group_of(c), g.title);
            }
        }
    }
}

#[test]
fn at_b5_atalhos_alternativos_numa_tabela_so() {
    let groups = sheet(Platform::Windows);
    assert_eq!(keys(&groups, "Refazer"), ["Ctrl+Y", "Ctrl+Shift+Z"]);
    assert_eq!(
        keys(&groups, "Tirar foto ou apagar quadro"),
        ["Delete", "Backspace"]
    );
    assert_eq!(keys(&groups, "Mais zoom"), ["+", "="]);
    assert_eq!(keys(&groups, "Aproximar a lâmina"), ["+", "="]);
    assert_eq!(keys(&groups, "Atalhos do teclado…"), ["Ctrl+/", "F1"]);
    assert_eq!(keys(&groups, "Próxima lâmina"), ["Page Down", "→"]);
    assert_eq!(keys(&groups, "Lâmina anterior"), ["Page Up", "←"]);
    // ← e → só são alternativos fora do modo organizar: lá elas movem a seleção.
    let right = Shortcut::key(Key::Right);
    assert_eq!(main_of(right, Mode::Normal), Shortcut::key(Key::PageDown));
    assert_eq!(main_of(right, Mode::Organize), right);
    for (alias, main) in NORMAL_ALIASES {
        assert_eq!(
            Command::from_shortcut_in(alias, Mode::Normal),
            Command::from_shortcut_in(main, Mode::Normal),
            "{alias:?}"
        );
        assert_ne!(
            Command::from_shortcut_in(alias, Mode::Organize),
            Command::from_shortcut_in(main, Mode::Organize),
            "{alias:?}"
        );
    }
    // Cada alternativo leva ao comando do principal, no modo em que ele vale.
    for (alias, main) in ALIASES {
        for mode in [Mode::Normal, Mode::Organize] {
            assert_eq!(
                Command::from_shortcut_in(alias, mode),
                Command::from_shortcut_in(main, mode),
                "{alias:?} em {mode:?}"
            );
        }
    }
    assert_eq!(
        Command::from_shortcut_in(Shortcut::key(Key::Char('=')), Mode::Organize),
        Some(Command::ViewZoomIn)
    );
    assert_eq!(
        Command::from_shortcut(Shortcut::ctrl_shift('z')),
        Some(Command::Redo)
    );
}

#[test]
fn at_b7_a_mesma_lista_em_qualquer_momento() {
    let platform = Platform::current();
    let mut lists = Vec::new();
    let empty = Editor::new();
    assert!(empty.session().is_none());
    lists.push(sheet(platform));
    let mut editor = editor_at(three_spreads(), 1);
    lists.push(sheet(platform));
    assert!(editor.run(Command::Organize));
    lists.push(sheet(platform));
    assert!(editor.run(Command::Organize), "sai do modo");
    editor.select_frame(Some(0));
    assert!(editor.run(Command::CropMode));
    lists.push(sheet(platform));
    assert!(lists.windows(2).all(|w| w[0] == w[1]));
    let full = sheet(platform);
    assert_eq!(search(&full, "foto"), search(&full, "foto"));
}

#[test]
fn at_b10_nenhum_atalho_se_repete() {
    for mode in [Mode::Normal, Mode::Organize] {
        let mains: Vec<Shortcut> = Command::ALL
            .into_iter()
            .filter(|c| c.in_mode(mode))
            .filter_map(Command::shortcut)
            .collect();
        let unique: HashSet<Shortcut> = mains.iter().copied().collect();
        assert_eq!(unique.len(), mains.len(), "atalho repetido em {mode:?}");
    }
    let all: Vec<Shortcut> = Command::ALL
        .into_iter()
        .filter_map(Command::shortcut)
        .collect();
    for (alias, main) in ALIASES {
        assert!(!all.contains(&alias), "{alias:?} é atalho principal");
        assert!(
            all.contains(&main),
            "{main:?} não é atalho de comando nenhum"
        );
    }
    // Os alternativos de um modo só (← e → fora do modo organizar) não são atalho principal
    // naquele modo, e levam a um atalho que vale nele.
    let normal: Vec<Shortcut> = Command::ALL
        .into_iter()
        .filter(|c| c.in_mode(Mode::Normal))
        .filter_map(Command::shortcut)
        .collect();
    for (alias, main) in NORMAL_ALIASES {
        assert!(!normal.contains(&alias), "{alias:?} é atalho principal");
        assert!(normal.contains(&main), "{main:?} não vale fora do modo");
    }
    // Alt+setas e Espaço ficam fora da tabela (a lista só os mostra).
    for key in [Key::Left, Key::Right, Key::Up, Key::Down, Key::Char(' ')] {
        for shift in [false, true] {
            let s = Shortcut {
                ctrl: false,
                shift,
                alt: key != Key::Char(' '),
                key,
            };
            assert!(!all.contains(&s), "{s:?}");
        }
    }
}

#[test]
fn at_b12_alt_setas_fazem_o_que_a_lista_diz() {
    let mut editor = editor_at(super::tl_common::three_spreads_whole(), 1);
    editor.select_frame(Some(0));
    let rect = |e: &Editor| {
        let s = e.session().unwrap();
        crate::hand::frame_mm(s.project(), s.current(), 0).unwrap()
    };
    let start = rect(&editor);
    let alt = |key, shift| Shortcut {
        ctrl: false,
        shift,
        alt: true,
        key,
    };
    let groups = sheet(Platform::Windows);
    assert_eq!(
        keys(&groups, "Mover o quadro selecionado 1 mm"),
        ["Alt+setas"]
    );
    assert!(editor.nudge_key(alt(Key::Right, false)));
    assert!(editor.nudge_key(alt(Key::Down, true)));
    let end = rect(&editor);
    assert!((end.x.get() - start.x.get() - 1.0).abs() < 1e-6);
    assert!((end.y.get() - start.y.get() - 10.0).abs() < 1e-6);
    assert_eq!(end.width, start.width);
    let s = editor.session().unwrap();
    assert_eq!(s.undo_label(), Some("Mover quadro"));
    assert_eq!(s.history_list().steps.len(), 1, "numa edição só");
}
