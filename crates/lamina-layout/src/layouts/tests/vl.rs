//! Cenários da spec `specs/variedade-de-layouts.md` (VL-B1 a VL-B8). VL-B9 a VL-B13 ficam em
//! `vl_score.rs`; a integridade, em `vl_integrity.rs`.

use std::collections::HashSet;

use lamina_core::{ActivePages, AlbumSpec, DesignMode, Dpi, Mm, RelRect};

use super::rotation::rotation_slot;
use super::*;

const H: f64 = 1.5;
const V: f64 = 2.0 / 3.0;
const Q: f64 = 1.0;
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

fn photos(h: usize, v: usize) -> Vec<f64> {
    // Verticais espalhadas: uma a cada três fotos, as que sobrarem no fim.
    let mut out = Vec::new();
    let (mut h, mut v) = (h, v);
    while h + v > 0 {
        if v > 0 && (out.len() % 3 == 2 || h == 0) {
            out.push(V);
            v -= 1;
        } else {
            out.push(H);
            h -= 1;
        }
    }
    out
}

fn total(l: &LayoutLists) -> usize {
    l.in_order.len() + l.reordered.len()
}

fn all(l: &LayoutLists) -> impl Iterator<Item = &LayoutCandidate> {
    l.in_order.iter().chain(&l.reordered)
}

fn crosses_gutter(f: &RelRect) -> bool {
    f.x() < 0.5 - 1e-9 && f.x() + f.width() > 0.5 + 1e-9
}

#[test]
fn vl_b1_muitas_opcoes_para_uma_foto() {
    let l = lists(DesignMode::Spread, BOTH, &[H]);
    assert!(l.in_order.len() >= 30, "{} opções", l.in_order.len());
    let widths: HashSet<u64> = l
        .in_order
        .iter()
        .map(|c| (c.frames[0].width() * 1e4).round().to_bits())
        .collect();
    let xs: HashSet<u64> = l
        .in_order
        .iter()
        .map(|c| (c.frames[0].x() * 1e4).round().to_bits())
        .collect();
    let margins: HashSet<u8> = l
        .in_order
        .iter()
        .map(|c| c.breathing.margin_level)
        .collect();
    assert!(
        widths.len() >= 3 && xs.len() >= 3,
        "tamanhos {widths:?} posições {xs:?}"
    );
    assert_eq!(margins.len(), 4);
}

#[test]
fn vl_b2_a_quantidade_acompanha_a_selecao() {
    let table: [(&[f64], ActivePages, usize); 11] = [
        (&[H], BOTH, 30),
        (&[V], BOTH, 30),
        (&[H, H], BOTH, 50),
        (&[H, V], BOTH, 60),
        (&[H; 4], BOTH, 100),
        (&[H, V, H, V], BOTH, 150),
        (&photos(6, 2), BOTH, 150),
        (&photos(7, 2), BOTH, 150),
        (&[H; 12], BOTH, 100),
        (&photos(9, 3), BOTH, 150),
        (&[H; 12], RIGHT, 30),
    ];
    for (aspects, active, min) in table {
        let l = lists(DesignMode::Spread, active, aspects);
        assert!(
            total(&l) >= min,
            "{} fotos: {} < {min}",
            aspects.len(),
            total(&l)
        );
    }
    let hh = lists(DesignMode::Spread, BOTH, &[H, H]);
    let hv = lists(DesignMode::Spread, BOTH, &[H, V]);
    let hhh = lists(DesignMode::Spread, BOTH, &[H, H, H]);
    assert_ne!(hh.in_order[0].frames, hv.in_order[0].frames);
    assert_ne!(hh.in_order.len(), hhh.in_order.len());
}

#[test]
fn vl_b3_reordenadas_so_com_orientacoes_misturadas() {
    assert!(
        lists(DesignMode::Spread, BOTH, &[H; 4])
            .reordered
            .is_empty()
    );
    let aspects = [H, H, V, V];
    let l = lists(DesignMode::Spread, BOTH, &aspects);
    assert!(!l.reordered.is_empty());
    for c in &l.reordered {
        assert_ne!(c.order, [0, 1, 2, 3]);
        let pos = |i: usize| c.order.iter().position(|&p| p == i).unwrap();
        assert!(
            pos(0) < pos(1) && pos(2) < pos(3),
            "ordem interna mudou: {:?}",
            c.order
        );
    }
}

#[test]
fn sem_reordenadas_a_lista_reordenada_vem_vazia() {
    let options = LayoutOptions {
        reordered: false,
        ..LayoutOptions::default()
    };
    let l = layout_options(&spec(DesignMode::Spread), BOTH, &[H, H, V, V], &options).unwrap();
    assert!(l.reordered.is_empty());
    assert_eq!(
        l.in_order,
        lists(DesignMode::Spread, BOTH, &[H, H, V, V]).in_order
    );
}

#[test]
fn vl_b4_lamina_inteira_justificada() {
    let l = lists(DesignMode::Spread, BOTH, &photos(6, 2));
    let band = l.in_order.iter().find(|c| {
        matches!(&c.family, Family::WholeSpread(p) if p.arrangement == Arrangement::Columns)
            && c.frames.iter().any(crosses_gutter)
            && c.crop.iter().all(|x| *x == 0.0)
    });
    assert!(band.is_some(), "falta a faixa de colunas pela lâmina");
    for c in all(&l) {
        for f in c.frames.iter().filter(|f| crosses_gutter(f)) {
            assert!(
                f.width() * 600.0 >= 75.0 - 1e-6,
                "quadro estreito na lombada: {f:?}"
            );
        }
    }
}

#[test]
fn vl_b5_recorte_so_onde_e_permitido() {
    for aspects in [vec![H], vec![V], vec![H, V, Q], photos(3, 1), photos(6, 2)] {
        let l = lists(DesignMode::Spread, BOTH, &aspects);
        for c in all(&l) {
            let worst = c.crop.iter().copied().fold(0.0, f64::max);
            let limit = match c.family.kind() {
                FamilyKind::Across | FamilyKind::BleedPage | FamilyKind::FullBleed => 0.35,
                FamilyKind::Mosaic => 0.15,
                _ => 0.0,
            };
            assert!(worst <= limit + 1e-9, "{:?}: recorte {worst}", c.family);
        }
    }
}

/// Nas 12 primeiras: uma família só repete depois de todas as que ainda vêm na lista (com nota
/// de pelo menos 60% da primeira) terem aparecido, e nenhum desenho geral se repete.
fn assert_rotation(list: &[LayoutCandidate]) {
    let floor = list[0].score.total * 0.6;
    let mut round: HashSet<(FamilyKind, u8)> = HashSet::new();
    for (i, c) in list.iter().take(12).enumerate() {
        let kind = rotation_slot(&c.family);
        assert!(
            !list[..i]
                .iter()
                .any(|shown| shown.family.same_outline(&c.family)),
            "posição {}: arranjo repetido",
            i + 1
        );
        if round.contains(&kind) {
            let ahead: HashSet<(FamilyKind, u8)> = list[i..]
                .iter()
                .filter(|c| {
                    c.score.total >= floor
                        && !list[..i].iter().any(|s| s.family.same_outline(&c.family))
                })
                .map(|c| rotation_slot(&c.family))
                .collect();
            assert!(
                ahead.is_subset(&round),
                "posição {}: {kind:?} repetiu cedo",
                i + 1
            );
            round.clear();
        }
        round.insert(kind);
    }
}

#[test]
fn vl_b6_variedade_logo_nas_primeiras_sugestoes() {
    let l = lists(DesignMode::Spread, BOTH, &[H, H, V, V]);
    assert_rotation(&l.in_order);
    assert_rotation(&l.reordered);
    let first: HashSet<FamilyKind> = l
        .in_order
        .iter()
        .take(10)
        .map(|c| c.family.kind())
        .collect();
    assert!(first.len() >= 3, "{first:?}");
}

#[test]
fn vl_b7_espelhado() {
    let l = lists(DesignMode::Spread, BOTH, &[H, V, H, V]);
    let mirrored = l.in_order.iter().find(|c| {
        let mirror = |a: &RelRect, b: &RelRect| {
            (b.x() - (1.0 - a.x() - a.width())).abs() < 1e-9 && (a.y() - b.y()).abs() < 1e-9
        };
        matches!(c.family, Family::Mirrored(_))
            && mirror(&c.frames[0], &c.frames[2])
            && mirror(&c.frames[1], &c.frames[3])
    });
    assert!(mirrored.is_some(), "falta o espelhado");
}

#[test]
fn vl_b8_modo_pagina_nao_cruza_a_lombada() {
    let l = lists(DesignMode::Page, BOTH, &photos(6, 2));
    assert!(total(&l) > 0);
    for c in all(&l) {
        assert!(!c.frames.iter().any(crosses_gutter), "{:?}", c.family);
    }
}
