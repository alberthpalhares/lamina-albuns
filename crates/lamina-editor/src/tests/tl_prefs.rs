//! Espelhar, favoritos, rejeitados, nota, preferências, comandos e velocidade (TL-B6 a B8, B10 a
//! B13; o TL-B9 está no `lamina-layout`).

use std::time::{Duration, Instant};

use lamina_core::testing::budget;
use lamina_core::{Crop, FrameContent, Project};
use lamina_layout::{LayoutOptions, design_key, layout_options};

use super::tl_common::{H, V, built, editor_at, frames, single_start, spec, three_spreads};
use crate::text::ALL_REJECTED;
use crate::{Command, Editor, Key, Part, PrefsStore, Shortcut, score_text};

#[test]
fn tl_b6_espelhar() {
    let mut editor = editor_at(three_spreads(), 1);
    let before = frames(&editor);
    assert!(editor.run(Command::Mirror));
    let mirrored = frames(&editor);
    for ((p0, r0), (p1, r1)) in before.iter().zip(&mirrored) {
        assert_eq!(p0, p1, "mesma foto");
        assert!((r1.x() - (1.0 - r0.x() - r0.width())).abs() < 1e-12);
        assert!((r1.y() - r0.y()).abs() < 1e-12);
    }
    assert!(editor.run(Command::Mirror));
    for ((p0, r0), (p2, r2)) in before.iter().zip(&frames(&editor)) {
        assert_eq!(p0, p2);
        assert!((r0.x() - r2.x()).abs() < 1e-12 && (r0.width() - r2.width()).abs() < 1e-12);
    }
    let edge = built(single_start(spec(30.0, 30.0)), &[H, H, H, V, H, H], 2);
    let editor = editor_at(edge, 0);
    assert!(
        !editor.available(Command::Mirror),
        "ponta com página simples"
    );
}

fn hhv(width_cm: f64) -> Project {
    built(spec(width_cm, 30.0), &[H, H, V], 1)
}

#[test]
fn tl_b7_favorito_no_topo_em_outro_album() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("prefs.json");
    let mut editor = Editor::with_prefs(PrefsStore::load(&file));
    editor.adopt(hhv(30.0));
    assert!(editor.apply_option(Part::Spread, 6));
    let panel = editor.panel(Part::Spread);
    let key = design_key(
        &panel.options[panel.in_use.unwrap()],
        &panel.selection.unwrap().aspects,
    );
    assert!(editor.run(Command::Favorite));

    let mut other = Editor::with_prefs(PrefsStore::load(&file));
    other.adopt(hhv(25.0));
    let panel = other.panel(Part::Spread);
    let aspects = panel.selection.clone().unwrap().aspects;
    assert_eq!(design_key(&panel.options[0], &aspects), key);
    assert!(panel.favorite[0]);
}

#[test]
fn tl_b8_rejeitado_some() {
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H], 1), 0);
    let panel = editor.panel(Part::Spread);
    let (p, aspects) = (panel.in_use.unwrap(), panel.selection.unwrap().aspects);
    let key = design_key(&panel.options[p], &aspects);
    let following = design_key(&panel.options[p + 1], &aspects);
    assert!(editor.run(Command::Reject));
    let panel = editor.panel(Part::Spread);
    assert!(panel.options.iter().all(|c| design_key(c, &aspects) != key));
    let now = design_key(&panel.options[panel.in_use.unwrap()], &aspects);
    assert_eq!(now, following, "passou para a seguinte");
    let total = panel.options.len() + 1;
    for _ in 0..total {
        if editor.panel(Part::Spread).all_rejected {
            break;
        }
        assert!(editor.run(Command::Reject));
    }
    let panel = editor.panel(Part::Spread);
    assert!(panel.all_rejected);
    assert_eq!(panel.options.len(), total, "a lista volta inteira");
    assert_eq!(
        ALL_REJECTED,
        "Todas as opções desta seleção foram rejeitadas."
    );
}

#[test]
fn estrela_e_x_da_miniatura() {
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H, H], 1), 0);
    let panel = editor.panel(Part::Spread);
    let aspects = panel.selection.clone().unwrap().aspects;
    let in_use = panel.in_use.unwrap();
    let other = usize::from(in_use == 0) + 2;
    let key = design_key(&panel.options[other], &aspects);
    assert!(editor.mark_option(other, true));
    let panel = editor.panel(Part::Spread);
    assert_eq!(
        design_key(&panel.options[0], &aspects),
        key,
        "favorita sobe"
    );
    assert!(editor.mark_option(0, false));
    let panel = editor.panel(Part::Spread);
    assert!(
        panel.options.iter().all(|c| design_key(c, &aspects) != key),
        "rejeitada some"
    );
}

#[test]
fn tl_b10_nota_explicada() {
    let editor = editor_at(three_spreads(), 1);
    for option in editor.panel(Part::Spread).options {
        let text = score_text(&option.score);
        let numbers: Vec<i64> = text
            .split(|c: char| !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
            .map(|t| t.parse().unwrap())
            .collect();
        let [total, c, s, b, crop] = numbers[..] else {
            panic!("{text}");
        };
        assert_eq!(total, c + s + b - crop, "{text}");
        assert!((total as f64 - option.score.total).abs() <= 2.0, "{text}");
    }
}

#[test]
fn tl_b11_preferencias_ilegiveis() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("prefs.json");
    std::fs::write(&file, "{ quebrado").unwrap();
    let store = PrefsStore::load(&file);
    assert!(store.prefs.is_empty());
    let mut editor = Editor::with_prefs(store);
    let notice = editor.take_notice().unwrap();
    assert!(
        notice.starts_with("Não foi possível ler as preferências de layout: "),
        "{notice}"
    );
    assert!(notice.ends_with('.'));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "{ quebrado",
        "não sobrescreve"
    );
}

#[test]
fn tl_b12_registro_de_comandos() {
    let shortcuts: Vec<Shortcut> = Command::ALL.iter().filter_map(|c| c.shortcut()).collect();
    let unique: std::collections::HashSet<_> = shortcuts.iter().collect();
    assert_eq!(unique.len(), shortcuts.len(), "atalhos repetidos");
    let key = |k| Command::from_shortcut(Shortcut::key(k));
    assert_eq!(key(Key::Right), Some(Command::NextLayout));
    assert_eq!(key(Key::Left), Some(Command::PreviousLayout));
    assert_eq!(key(Key::Down), Some(Command::NextLeft));
    assert_eq!(
        Command::from_shortcut(Shortcut::shift(Key::Up)),
        Some(Command::PreviousRight)
    );
    assert_eq!(
        Command::from_shortcut(Shortcut::ctrl('z')),
        Some(Command::Undo)
    );
    assert_eq!(
        Command::from_shortcut(Shortcut::ctrl_shift('z')),
        Some(Command::Redo)
    );
    let editor = Editor::new();
    for command in [
        Command::NextLayout,
        Command::NextLeft,
        Command::Mirror,
        Command::Favorite,
        Command::Undo,
    ] {
        assert!(!editor.available(command), "{} sem projeto", command.name());
    }
}

#[test]
fn tl_b13_trocar_rapido() {
    // 12 fotos de orientações misturadas numa lâmina só.
    let sizes: Vec<(u32, u32)> = (0..12).map(|i| if i % 3 == 2 { V } else { H }).collect();
    let mut project = Project::new("teste", spec(30.0, 30.0));
    let ids: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(i, (w, h))| project.add_photo(format!("f{i}.jpg"), *w, *h))
        .collect();
    let aspects: Vec<f64> = sizes
        .iter()
        .map(|(w, h)| f64::from(*w) / f64::from(*h))
        .collect();
    let both = lamina_core::ActivePages {
        left: true,
        right: true,
    };
    let first = layout_options(&project.spec, both, &aspects, &LayoutOptions::default())
        .unwrap()
        .in_order
        .remove(0);
    let spread = project.add_spread();
    for (id, rect) in ids.iter().zip(&first.frames) {
        let content = FrameContent::Photo {
            photo: *id,
            crop: Crop::default(),
        };
        project.add_frame(spread, *rect, content).unwrap();
    }
    let (mut lists, mut change) = (Duration::MAX, Duration::MAX);
    for _ in 0..3 {
        let mut editor = editor_at(project.clone(), 0);
        let started = Instant::now();
        let panel = editor.panel(Part::Spread);
        lists = lists.min(started.elapsed());
        assert!(panel.options.len() > 10);
        let started = Instant::now();
        assert!(editor.apply_option(Part::Spread, 3));
        change = change.min(started.elapsed());
    }
    assert!(lists < budget(200), "listas em {lists:?}");
    assert!(change < budget(50), "troca em {change:?}");
}
