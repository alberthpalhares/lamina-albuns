//! Sugestões, trocar pelo clique e pelas setas, uma página só e reordenadas (TL-B1 a B5).

use lamina_core::{FrameContent, Orientation};
use lamina_layout::{Family, LayoutOptions, layout_options};

use super::tl_common::{H, V, built, editor_at, frames, page, spec, three_spreads};
use crate::suggestions::{Unavailable, selection};
use crate::{Command, Part};

#[test]
fn tl_b1_sugestoes_da_lamina_atual() {
    let photos = [H, H, H, V];
    let mut project = lamina_core::Project::new("teste", spec(30.0, 30.0));
    for (i, (w, h)) in photos.iter().enumerate() {
        project.add_photo(format!("f{i:02}.jpg"), *w, *h);
    }
    let built = lamina_layout::auto_build(&project, 1, &Default::default()).unwrap();
    let index = 0;
    let editor = editor_at(built.project.clone(), index);
    let panel = editor.panel(Part::Spread);
    let sel = selection(&built.project, index, Part::Spread).unwrap();
    assert_eq!(sel.photos.len(), 4);
    let expected = layout_options(
        &built.project.spec,
        sel.active,
        &sel.aspects,
        &LayoutOptions::default(),
    )
    .unwrap()
    .in_order;
    assert_eq!(panel.options, expected);
    assert_eq!(
        panel.in_use,
        Some(built.report[index].position - 1),
        "a escolha do Auto Build"
    );
}

#[test]
fn tl_b2_trocar_pelo_clique() {
    let project = three_spreads();
    let mut editor = editor_at(project.clone(), 1);
    let before = project.spreads[1].clone();
    assert!(editor.apply_option(Part::Spread, 4));
    let panel = editor.panel(Part::Spread);
    assert_eq!(panel.in_use, Some(4));
    let chosen = &panel.options[4];
    let sel = panel.selection.as_ref().unwrap();
    let now = frames(&editor);
    for (photo, rect) in sel.photos.iter().zip(&chosen.frames) {
        assert!(now.contains(&(*photo, *rect)), "foto no quadro da opção");
    }
    let s = editor.session().unwrap();
    for f in &s.project().spreads[1].frames {
        assert!(
            matches!(&f.content, FrameContent::Photo { crop, .. } if *crop == Default::default())
        );
    }
    assert!(s.is_dirty());
    s.project().validate().unwrap();
    // Desfazer em outra lâmina volta para a lâmina da edição.
    assert!(editor.run(Command::FirstSpread));
    assert!(editor.run(Command::Undo));
    let s = editor.session().unwrap();
    assert_eq!(s.current(), 1);
    assert_eq!(s.project().spreads[1], before);
    assert!(editor.run(Command::Redo));
    assert_eq!(editor.panel(Part::Spread).in_use, Some(4));
}

#[test]
fn tl_b3_setas_na_lamina_inteira() {
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.apply_option(Part::Spread, 0));
    let at = |e: &crate::Editor| e.panel(Part::Spread).in_use;
    assert!(editor.run(Command::NextLayout));
    assert_eq!(at(&editor), Some(1));
    assert!(editor.run(Command::NextLayout));
    assert_eq!(at(&editor), Some(2));
    assert!(editor.run(Command::PreviousLayout));
    assert_eq!(at(&editor), Some(1));
    assert!(editor.apply_option(Part::Spread, 0));
    assert!(editor.run(Command::PreviousLayout));
    let last = editor.panel(Part::Spread).options.len() - 1;
    assert_eq!(at(&editor), Some(last), "da 1ª, o ← vai para a última");
}

#[test]
fn tl_b4_uma_pagina_so() {
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H, H, H, V], 1), 0);
    // Uma opção com 2 fotos em cada página e nada sobre a lombada.
    let panel = editor.panel(Part::Spread);
    let two_two = panel.options.iter().position(|c| match &c.family {
        Family::PerPage {
            left: Some(l),
            right: Some(r),
        } => l.groups.iter().sum::<usize>() == 2 && r.groups.iter().sum::<usize>() == 2,
        _ => false,
    });
    assert!(editor.apply_option(Part::Spread, two_two.unwrap()));

    let right = page(&editor, false);
    let left = page(&editor, true);
    assert!(editor.run(Command::NextLeft));
    assert_eq!(page(&editor, false), right, "↓ não mexe na direita");
    assert_ne!(page(&editor, true), left, "↓ muda a esquerda");
    let left = page(&editor, true);
    assert!(editor.run(Command::NextRight));
    assert_eq!(page(&editor, true), left, "Shift+↓ não mexe na esquerda");
    assert_ne!(page(&editor, false), right);

    // Com uma foto sobre a lombada, só a lâmina inteira troca.
    let across = editor
        .panel(Part::Spread)
        .options
        .iter()
        .position(|c| {
            c.frames
                .iter()
                .any(|r| r.x() < 0.49 && r.x() + r.width() > 0.51)
        })
        .unwrap();
    assert!(editor.apply_option(Part::Spread, across));
    assert!(!editor.available(Command::NextLeft) && !editor.available(Command::NextRight));
    let reason = editor.panel(Part::Left).unavailable.unwrap();
    assert_eq!(reason, Unavailable::CrossesSpine);
    assert_eq!(
        reason.to_string(),
        "Esta lâmina tem foto sobre a lombada: troque o layout da lâmina inteira."
    );
}

#[test]
fn tl_b5_opcao_reordenada() {
    let project = built(spec(30.0, 30.0), &[H, H, V, V], 1);
    let mut editor = editor_at(project, 0);
    assert!(editor.run(Command::ToggleReordered));
    let panel = editor.panel(Part::Spread);
    assert!(!panel.options.is_empty());
    let pick = panel
        .options
        .iter()
        .position(|c| c.order != vec![0, 1, 2, 3])
        .unwrap();
    assert!(editor.apply_option(Part::Spread, pick));
    let s = editor.session().unwrap();
    let trim = s.project().spec.trim_rect();
    for (photo, rect) in frames(&editor) {
        let frame_aspect = rect.width() * trim.width.get() / (rect.height() * trim.height.get());
        let p = s.project().photo(photo).unwrap();
        let wanted = Orientation::from_aspect_ratio(p.aspect_ratio());
        assert_eq!(
            Orientation::from_aspect_ratio(frame_aspect),
            wanted,
            "quadro da orientação certa"
        );
    }
}
