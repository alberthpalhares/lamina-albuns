//! Cenários da spec `novo-projeto` (NP) que travam a montagem no gerador: fotos inteiras,
//! recorte leve, sem lay-flat e a montagem guardada no projeto (NP-B7 a NP-B10).

use lamina_core::{ActivePages, CropLevel, DesignMode, FrameContent, Mounting, Project, RelRect};

use super::tests::{album, spec};
use super::*;
use crate::layouts::{FamilyKind, LayoutLists, LayoutOptions, layout_options};

const BOTH: ActivePages = ActivePages {
    left: true,
    right: true,
};

fn mounting(crop: CropLevel) -> Mounting {
    Mounting {
        crop,
        ..Mounting::default()
    }
}

/// Fração da foto que fica de fora de cada quadro de foto do projeto.
fn crops(project: &Project) -> Vec<f64> {
    let trim = project.spec.trim_rect();
    let (w, h) = (trim.width.get(), trim.height.get());
    let mut out = Vec::new();
    for frame in project.spreads.iter().flat_map(|s| &s.frames) {
        let FrameContent::Photo { photo, .. } = &frame.content else {
            continue;
        };
        let r = frame.rect;
        let frame_aspect = r.width() * w / (r.height() * h);
        let photo_aspect = project.photo(*photo).unwrap().aspect_ratio();
        let visible = (frame_aspect / photo_aspect).min(photo_aspect / frame_aspect);
        out.push(1.0 - visible);
    }
    out
}

fn panel(project: &Project, aspects: &[f64]) -> LayoutLists {
    let options = LayoutOptions {
        mounting: project.mounting,
        ..LayoutOptions::default()
    };
    layout_options(&project.spec, BOTH, aspects, &options).unwrap()
}

fn built(m: Mounting, mode: DesignMode) -> Project {
    let mut project = album(spec().with_mode(mode), 8, &(0..80).collect::<Vec<_>>());
    project.mounting = m;
    auto_build(&project, 20, &BuildOptions::default())
        .unwrap()
        .project
}

#[test]
fn np_b7_fotos_inteiras_nada_recorta() {
    let whole = mounting(CropLevel::Whole);
    let project = built(whole, DesignMode::Spread);
    assert_eq!(project.spreads.len(), 20);
    let worst = crops(&project).into_iter().fold(0.0, f64::max);
    assert!(worst < 1e-6, "recorte de {worst}");
    let lists = panel(&project, &[1.5, 1.5, 2.0 / 3.0, 1.5, 1.0]);
    for c in lists.in_order.iter().chain(&lists.reordered) {
        assert!(
            c.crop.iter().all(|&x| x < 1e-6),
            "{:?} {:?}",
            c.family,
            c.crop
        );
    }
}

#[test]
fn np_b7_livre_continua_recortando_como_antes() {
    let project = built(Mounting::default(), DesignMode::Spread);
    let lists = panel(&project, &[1.5]);
    let kinds: Vec<FamilyKind> = lists.in_order.iter().map(|c| c.family.kind()).collect();
    assert!(kinds.contains(&FamilyKind::Across), "{kinds:?}");
}

#[test]
fn np_b8_recorte_leve_ate_15_por_cento() {
    let light = mounting(CropLevel::Light);
    for aspects in [
        vec![1.5],
        vec![1.5, 2.0 / 3.0],
        vec![1.5; 5],
        vec![2.0 / 3.0; 6],
    ] {
        let lists = layout_options(
            &spec(),
            BOTH,
            &aspects,
            &LayoutOptions {
                mounting: light,
                ..LayoutOptions::default()
            },
        )
        .unwrap();
        for c in lists.in_order.iter().chain(&lists.reordered) {
            let worst = c.crop.iter().copied().fold(0.0, f64::max);
            assert!(worst <= 0.15 + 1e-9, "{:?} recorta {worst}", c.family);
        }
    }
}

#[test]
fn sb_b14_o_auto_build_varia() {
    let project = built(Mounting::default(), DesignMode::Spread);
    let families: Vec<Family> = auto_build(&project, 20, &BuildOptions::default())
        .unwrap()
        .report
        .iter()
        .map(|r| r.family.clone())
        .collect();
    let bleeds: Vec<&Family> = families
        .iter()
        .filter(|f| f.kind() == FamilyKind::FullBleed)
        .collect();
    assert!(!bleeds.is_empty(), "nenhuma lâmina sem borda");
    assert!(
        bleeds.iter().any(|f| !f.same_outline(bleeds[0])),
        "todas com o mesmo desenho: {bleeds:?}"
    );
    let edge = |f: &Family| {
        matches!(
            f.kind(),
            FamilyKind::Across | FamilyKind::BleedPage | FamilyKind::FullBleed
        )
    };
    assert!(
        !families.windows(3).any(|w| w.iter().all(edge)),
        "três lâminas seguidas até a borda: {families:?}"
    );
}

#[test]
fn np_b8_sem_pagina_sangrada_nem_mosaicos() {
    let off = Mounting {
        bleed_page: false,
        mosaics: false,
        ..Mounting::default()
    };
    let options = LayoutOptions {
        mounting: off,
        ..LayoutOptions::default()
    };
    let lists = layout_options(&spec(), BOTH, &[1.5; 6], &options).unwrap();
    for c in lists.in_order.iter().chain(&lists.reordered) {
        let kind = c.family.kind();
        assert!(
            kind != FamilyKind::BleedPage
                && kind != FamilyKind::Mosaic
                && kind != FamilyKind::FullBleed,
            "{kind:?}"
        );
    }
}

/// O quadro passa pela dobra (o meio da lâmina)?
fn crosses_fold(r: &RelRect) -> bool {
    r.x() < 0.5 - 1e-9 && r.x() + r.width() > 0.5 + 1e-9
}

#[test]
fn np_b9_sem_lay_flat_nada_atravessa_a_dobra() {
    let project = built(Mounting::default(), DesignMode::Page);
    let crossing = project
        .spreads
        .iter()
        .flat_map(|s| &s.frames)
        .filter(|f| crosses_fold(&f.rect))
        .count();
    assert_eq!(crossing, 0);
    // Com lay-flat, o mesmo álbum tem quadros atravessando (a trava não é vazia).
    let flat = built(Mounting::default(), DesignMode::Spread);
    assert!(
        flat.spreads
            .iter()
            .flat_map(|s| &s.frames)
            .any(|f| crosses_fold(&f.rect))
    );
}

#[test]
fn np_b10_a_montagem_vai_e_volta_do_arquivo() {
    let m = Mounting {
        crop: CropLevel::Whole,
        mosaics: false,
        ..Mounting::default()
    };
    let project = built(m, DesignMode::Spread);
    let back = Project::from_json(&project.to_json().unwrap()).unwrap();
    assert_eq!(back.mounting, m);
    let lists = panel(&back, &[1.5, 1.5, 1.5, 2.0 / 3.0]);
    for c in lists.in_order.iter().chain(&lists.reordered) {
        assert!(c.crop.iter().all(|&x| x < 1e-6));
        assert_ne!(c.family.kind(), FamilyKind::Mosaic);
    }
}
