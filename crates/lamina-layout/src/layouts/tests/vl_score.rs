//! Cenários VL-B9 a VL-B13 da spec `specs/variedade-de-layouts.md`: velocidade, 12 fotos numa
//! página, limite, nota e menor foto.

use std::time::{Duration, Instant};

use lamina_core::{ActivePages, AlbumSpec, DesignMode, Dpi, Mm, Rect};

use super::area::Ctx;
use super::score::score;
use super::*;
use lamina_core::testing::budget;

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

fn lists(mode: DesignMode, active: ActivePages, aspects: &[f64]) -> LayoutLists {
    layout_options(&spec(mode), active, aspects, &LayoutOptions::default()).unwrap()
}

/// `h` horizontais e `v` verticais, com as verticais espalhadas.
fn photos(h: usize, v: usize) -> Vec<f64> {
    let mut out = vec![H; h];
    for i in 0..v {
        out.insert((i * 3 + 2).min(out.len()), V);
    }
    out
}

fn fastest(aspects: &[f64]) -> Duration {
    let s = spec(DesignMode::Spread);
    (0..3)
        .map(|_| {
            let start = Instant::now();
            layout_options(&s, BOTH, aspects, &LayoutOptions::default()).unwrap();
            start.elapsed()
        })
        .min()
        .unwrap()
}

#[test]
fn vl_b9_rapido_o_bastante_para_a_interface() {
    let twelve = fastest(&photos(9, 3));
    assert!(twelve < budget(200), "12 fotos: {twelve:?}");
    let many = fastest(&photos(18, 6));
    assert!(many < budget(1000), "24 fotos: {many:?}");
}

#[test]
fn vl_b10_doze_fotos_numa_pagina() {
    let l = lists(DesignMode::Spread, RIGHT, &[H; 12]);
    assert!(l.in_order.len() >= 30, "{}", l.in_order.len());
    for c in &l.in_order {
        assert!(
            c.frames.iter().all(|f| f.x() >= 0.5),
            "fora da página ativa"
        );
        // Toda divisão com mais de 6 fotos é equilibrada, em qualquer família (auditoria do
        // Marco 1, M10: antes só "por página" era conferida).
        let layout = match &c.family {
            Family::PerPage { right: Some(p), .. }
            | Family::WholeSpread(p)
            | Family::Mirrored(p)
            | Family::Mosaic { layout: p, .. }
            | Family::Hero { layout: p, .. } => Some(p),
            _ => None,
        };
        if let Some(p) = layout.filter(|p| p.groups.iter().sum::<usize>() > 6) {
            let (min, max) = (
                p.groups.iter().min().unwrap(),
                p.groups.iter().max().unwrap(),
            );
            assert!(max - min <= 1, "divisão desequilibrada: {:?}", p.groups);
        }
    }
}

#[test]
fn vl_b11_limite_de_fotos() {
    let err = layout_options(
        &spec(DesignMode::Spread),
        BOTH,
        &[H; 25],
        &LayoutOptions::default(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        LayoutError::TooManyPhotos {
            photos: 25,
            pages: 2,
            capacity: 24
        }
    ));
    assert_eq!(
        err.to_string(),
        "São 25 fotos para 2 páginas: cabem no máximo 24."
    );
}

#[test]
fn vl_b12_a_nota_explica_a_ordem() {
    let l = lists(DesignMode::Spread, BOTH, &[H; 4]);
    for c in &l.in_order {
        let s = c.score;
        let sum = (s.coverage + s.smallest + s.balance + s.crop).clamp(0.0, 100.0);
        assert!((s.total - sum).abs() < 1e-9, "{s:?}");
    }
    assert!(l.in_order.len() > 13);
    for pair in l.in_order[12..].windows(2) {
        assert!(
            pair[0].score.total + 1e-6 >= pair[1].score.total,
            "fora de ordem"
        );
    }
}

#[test]
fn vl_b13_a_foto_minuscula_perde_pontos() {
    let s = spec(DesignMode::Spread);
    let ctx = Ctx {
        spec: &s,
        active: RIGHT,
        gap: 0.0,
        inset: 0.0,
    };
    let r = |x: f64, w: f64, h: f64| Rect::new(Mm(x), Mm(20.0), Mm(w), Mm(h));
    // Mesma área total (30 000 mm²): três de 100 × 100, ou uma de 150 × 150 e duas de 75 × 50.
    let even = [
        r(310.0, 100.0, 100.0),
        r(420.0, 100.0, 100.0),
        r(310.0, 100.0, 100.0),
    ];
    let uneven = [
        r(310.0, 150.0, 150.0),
        r(470.0, 75.0, 50.0),
        r(470.0, 75.0, 50.0),
    ];
    let useful = 280.0 * 280.0;
    let a = score(&ctx, &even, &[0.0; 3], useful);
    let b = score(&ctx, &uneven, &[0.0; 3], useful);
    assert!((a.coverage - b.coverage).abs() < 1e-9);
    assert!(a.total > b.total, "{a:?} {b:?}");
}
