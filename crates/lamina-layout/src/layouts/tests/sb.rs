//! Cenários SB-B1 a SB-B10 e SB-B15 da spec `specs/laminas-sem-borda.md`.

use lamina_core::{ActivePages, AlbumSpec, CropLevel, DesignMode, Dpi, Mm, Mounting, Rect};

use super::area::Ctx;
use super::score::score;
use super::*;
use crate::prefs::design_key;

const H: f64 = 1.5;
const V: f64 = 2.0 / 3.0;
const BOTH: ActivePages = ActivePages {
    left: true,
    right: true,
};
const RIGHT: ActivePages = ActivePages {
    left: false,
    right: true,
};

fn spec(mode: DesignMode) -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
        .with_mode(mode)
}

fn lists(s: &AlbumSpec, active: ActivePages, aspects: &[f64], o: LayoutOptions) -> LayoutLists {
    layout_options(s, active, aspects, &o).unwrap()
}

/// As opções sem borda, de toda a lista: o teto de 200 não pode escondê-las do teste.
fn bleeds(s: &AlbumSpec, active: ActivePages, aspects: &[f64], gap: Mm) -> Vec<LayoutCandidate> {
    let options = LayoutOptions {
        gap,
        max_candidates: usize::MAX,
        ..LayoutOptions::default()
    };
    lists(s, active, aspects, options)
        .in_order
        .into_iter()
        .filter(|c| c.family.kind() == FamilyKind::FullBleed)
        .collect()
}

fn rects(s: &AlbumSpec, c: &LayoutCandidate) -> Vec<Rect> {
    c.frames
        .iter()
        .map(|f| f.to_absolute(&s.trim_rect()))
        .collect()
}

/// Encosta nas quatro bordas de `area`?
fn touches_all(frames: &[Rect], area: &Rect) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
    frames.iter().any(|r| near(r.x.get(), area.x.get()))
        && frames.iter().any(|r| near(r.y.get(), area.y.get()))
        && frames
            .iter()
            .any(|r| near(r.right().get(), area.right().get()))
        && frames
            .iter()
            .any(|r| near(r.bottom().get(), area.bottom().get()))
}

fn share(frames: &[Rect], area: &Rect) -> f64 {
    frames
        .iter()
        .map(|r| r.width.get() * r.height.get())
        .sum::<f64>()
        / (area.width.get() * area.height.get())
}

#[test]
fn sb_b1_ha_opcoes_sem_borda_de_2_a_12_fotos() {
    let s = spec(DesignMode::Spread);
    let trim = s.trim_rect();
    let mixed = [H, H, V, H, H, V, H, H, H, V, H, H];
    for n in [2, 3, 4, 6, 8, 12] {
        for aspects in [vec![H; n], mixed[..n].to_vec()] {
            let found = bleeds(&s, BOTH, &aspects, DEFAULT_GAP);
            assert!(!found.is_empty(), "sem opção com {aspects:?}");
            for c in &found {
                let frames = rects(&s, c);
                assert!(touches_all(&frames, &trim), "{:?}", c.family);
                assert!(share(&frames, &trim) >= 0.9, "{:?}", c.family);
            }
        }
    }
}

#[test]
fn sb_b2_o_nivel_de_recorte_manda() {
    let s = spec(DesignMode::Spread);
    let with = |crop| LayoutOptions {
        max_candidates: usize::MAX,
        mounting: Mounting {
            crop,
            ..Mounting::default()
        },
        ..LayoutOptions::default()
    };
    let in_family = |crop| {
        let l = lists(&s, BOTH, &[H; 4], with(crop));
        l.in_order
            .into_iter()
            .filter(|c| c.family.kind() == FamilyKind::FullBleed)
            .collect::<Vec<_>>()
    };
    assert!(in_family(CropLevel::Whole).is_empty());
    let worst = |list: &[LayoutCandidate]| {
        list.iter()
            .flat_map(|c| c.crop.iter().copied())
            .fold(0.0, f64::max)
    };
    let light = in_family(CropLevel::Light);
    assert!(worst(&light) <= 0.15 + 1e-9);
    let free = in_family(CropLevel::Free);
    assert!(!free.is_empty());
    assert!(worst(&free) <= 0.35 + 1e-9);
}

#[test]
fn sb_b3_sem_lay_flat_nada_cruza_a_dobra() {
    let s = spec(DesignMode::Page);
    let found = bleeds(&s, BOTH, &[H; 4], DEFAULT_GAP);
    assert!(!found.is_empty());
    for c in &found {
        assert!(matches!(
            c.family,
            Family::FullBleed(FullBleed::Pages { .. })
        ));
        for f in &c.frames {
            assert!(!(f.x() < 0.5 - 1e-9 && f.x() + f.width() > 0.5 + 1e-9));
        }
    }
}

#[test]
fn sb_b4_com_espaco_zero_cobre_tudo() {
    let s = spec(DesignMode::Spread);
    let trim = s.trim_rect();
    let found = bleeds(&s, BOTH, &[H; 4], Mm(0.0));
    assert!(!found.is_empty());
    for c in &found {
        assert!(
            (share(&rects(&s, c), &trim) - 1.0).abs() < 1e-6,
            "{:?}",
            c.family
        );
    }
}

#[test]
fn sb_b5_a_opcao_desligada_tira_a_familia() {
    let s = spec(DesignMode::Spread);
    let options = LayoutOptions {
        max_candidates: usize::MAX,
        mounting: Mounting {
            bleed_page: false,
            ..Mounting::default()
        },
        ..LayoutOptions::default()
    };
    let l = lists(&s, BOTH, &[H; 4], options);
    assert!(
        l.in_order
            .iter()
            .all(|c| c.family.kind() != FamilyKind::FullBleed)
    );
}

#[test]
fn sb_b6_pagina_unica_nao_recua() {
    let s = spec(DesignMode::Spread);
    let page = s.page_rect(lamina_core::PageSide::Right);
    let found = bleeds(&s, RIGHT, &[H; 4], DEFAULT_GAP);
    assert!(!found.is_empty());
    for c in &found {
        let frames = rects(&s, c);
        assert!(touches_all(&frames, &page), "{:?}", c.family);
        assert!(frames.iter().all(|r| r.x.get() >= page.x.get() - 1e-6));
    }
}

#[test]
fn sb_b7_o_fio_da_dobra_tem_a_largura_do_espaco() {
    let s = spec(DesignMode::Spread);
    let gutter = s.page_rect(lamina_core::PageSide::Right).x.get();
    let half = DEFAULT_GAP.get() / 2.0;
    let found = bleeds(&s, BOTH, &[H; 4], DEFAULT_GAP);
    let pages: Vec<_> = found
        .iter()
        .filter(|c| matches!(c.family, Family::FullBleed(FullBleed::Pages { .. })))
        .collect();
    assert!(!pages.is_empty());
    for c in pages {
        let frames = rects(&s, c);
        let left_edge = frames
            .iter()
            .filter(|r| r.right().get() < gutter)
            .map(|r| r.right().get())
            .fold(0.0, f64::max);
        let right_edge = frames
            .iter()
            .filter(|r| r.x.get() > gutter)
            .map(|r| r.x.get())
            .fold(f64::MAX, f64::min);
        assert!((gutter - left_edge - half).abs() < 1e-6, "{left_edge}");
        assert!((right_edge - gutter - half).abs() < 1e-6, "{right_edge}");
    }
}

#[test]
fn sb_b8_mais_branco_da_nota_menor() {
    let s = spec(DesignMode::Spread);
    let ctx = Ctx {
        spec: &s,
        active: BOTH,
        gap: 0.0,
        inset: 0.0,
    };
    let trim = s.trim_rect();
    let cut = trim.width.get() * trim.height.get();
    let (x0, y0) = (trim.x.get(), trim.y.get());
    let pair = |margin: f64| {
        let w = 300.0 - 2.0 * margin;
        let h = 300.0 - 2.0 * margin;
        [
            Rect::new(Mm(x0 + margin), Mm(y0 + margin), Mm(w), Mm(h)),
            Rect::new(Mm(x0 + 300.0 + margin), Mm(y0 + margin), Mm(w), Mm(h)),
        ]
    };
    let full = score(&ctx, &pair(0.0), &[0.0; 2], cut);
    // Encolher só até a zona segura já perde pontos: a borda agora conta.
    let safe = score(&ctx, &pair(10.0), &[0.0; 2], cut);
    let small = score(&ctx, &pair(70.0), &[0.0; 2], cut);
    assert!(full.total > safe.total, "{full:?} {safe:?}");
    assert!(safe.total > small.total, "{safe:?} {small:?}");
}

#[test]
fn sb_b9_fotos_maiores_abrem_a_lista() {
    let s = spec(DesignMode::Spread);
    let options = LayoutOptions {
        max_candidates: usize::MAX,
        ..LayoutOptions::default()
    };
    let one = lists(&s, BOTH, &[H], options);
    assert_eq!(one.in_order[0].family, Family::Across);
    for n in [2, 4, 8] {
        let l = lists(&s, BOTH, &vec![H; n], options);
        assert_eq!(
            l.in_order[0].family.kind(),
            FamilyKind::FullBleed,
            "{n} fotos: {:?}",
            l.in_order[0].family
        );
        for (i, c) in l.in_order.iter().enumerate() {
            if c.breathing.margin_level >= 2 {
                let same =
                    |o: &&LayoutCandidate| o.family == c.family && o.breathing.margin_level == 0;
                let earlier = l.in_order[..i].iter().find(same);
                let later = l.in_order[i..].iter().find(same);
                assert!(
                    later.is_none() || earlier.is_some(),
                    "{n} fotos: margem antes da versão sem margem: {:?}",
                    c.family
                );
            }
        }
    }
}

#[test]
fn sb_b10_as_chaves_antigas_nao_mudam() {
    let candidate = |family| LayoutCandidate {
        family,
        breathing: Breathing::default(),
        frames: Vec::new(),
        order: vec![0, 1],
        crop: Vec::new(),
        coverage: 0.0,
        score: Score::default(),
        model: None,
    };
    let rows = PageLayout {
        arrangement: Arrangement::Rows,
        groups: vec![2],
    };
    let key = |family| design_key(&candidate(family), &[H, H]);
    assert_eq!(key(Family::Across), "HH|atravessada|m0c");
    assert_eq!(key(Family::WholeSpread(rows.clone())), "HH|lamina:L2|m0c");
    assert_eq!(
        key(Family::Mosaic {
            scope: Scope::Spread,
            layout: rows.clone()
        }),
        "HH|mosaico:L:L2|m0c"
    );
    assert_eq!(
        key(Family::FullBleed(FullBleed::Spread(rows))),
        "HH|semborda:lamina:L2|m0c"
    );
}

#[test]
fn sb_b15_sem_cartoes_gemeos() {
    let s = spec(DesignMode::Spread);
    let found = bleeds(&s, BOTH, &[H; 2], DEFAULT_GAP);
    assert!(!found.is_empty());
    for (i, a) in found.iter().enumerate() {
        for b in &found[i + 1..] {
            assert_ne!(a.frames, b.frames, "{:?} {:?}", a.family, b.family);
        }
    }
}
