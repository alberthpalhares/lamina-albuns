//! Cenários da spec `specs/gerador-de-layouts.md` (TP-B1 a TP-B9) e o teste de integridade.

use lamina_core::{ActivePages, AlbumSpec, DesignMode, Dpi, Mm, Rect, RelRect};

use super::*;

const H: f64 = 1.5; // foto horizontal 3:2
const V: f64 = 2.0 / 3.0; // foto vertical 2:3
const Q: f64 = 1.0; // foto quadrada

const BOTH: ActivePages = ActivePages {
    left: true,
    right: true,
};
const RIGHT: ActivePages = ActivePages {
    left: false,
    right: true,
};

/// Álbum 30 × 30 cm, sem sangria, zona segura de 10 mm: a lâmina vai de 0 a 600 mm e a
/// lombada fica em 300 mm.
fn spec(mode: DesignMode) -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
        .with_mode(mode)
}

fn candidates(mode: DesignMode, active: ActivePages, aspects: &[f64]) -> Vec<LayoutCandidate> {
    layout_candidates(&spec(mode), active, aspects, DEFAULT_GAP).unwrap()
}

fn mm(spec: &AlbumSpec, frame: &RelRect) -> Rect {
    frame.to_absolute(&spec.trim_rect())
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn tp_b1_uma_foto_horizontal_numa_pagina() {
    let s = spec(DesignMode::Spread);
    let first = &candidates(DesignMode::Spread, RIGHT, &[H])[0];
    let f = mm(&s, &first.frames[0]);
    assert!(
        close(f.width.get(), 280.0) && close(f.height.get(), 186.67),
        "{f:?}"
    );
    assert!(
        close(f.x.get(), 310.0),
        "a 10 mm da borda da página direita: {f:?}"
    );
    assert!(
        close(f.y.get(), 10.0 + (280.0 - 186.67) / 2.0),
        "centralizada: {f:?}"
    );
}

#[test]
fn tp_b2_duas_fotos_lado_a_lado_com_o_espaco_padrao() {
    let s = spec(DesignMode::Spread);
    let all = candidates(DesignMode::Spread, RIGHT, &[H, H]);
    let row = all
        .iter()
        .find(|c| {
            matches!(&c.family, Family::PerPage { right: Some(p), .. }
                if p.arrangement == Arrangement::Rows && p.groups == [2])
        })
        .expect("candidato em uma linha");
    let (a, b) = (mm(&s, &row.frames[0]), mm(&s, &row.frames[1]));
    for f in [a, b] {
        assert!(
            close(f.width.get(), 138.59) && close(f.height.get(), 92.39),
            "{f:?}"
        );
    }
    assert!(
        close(b.x.get() - a.right().get(), 2.822),
        "espaço {a:?} {b:?}"
    );
}

/// Confere as regras 1, 2 e 5 em todos os candidatos de uma lâmina.
fn assert_valid(spec: &AlbumSpec, active: ActivePages, aspects: &[f64], all: &[LayoutCandidate]) {
    use lamina_core::PageSide;
    let gap = DEFAULT_GAP.get();
    let inside = |r: &Rect, area: &Rect| {
        r.x.get() >= area.x.get() - 1e-6
            && r.y.get() >= area.y.get() - 1e-6
            && r.right().get() <= area.right().get() + 1e-6
            && r.bottom().get() <= area.bottom().get() + 1e-6
    };
    for c in all {
        assert_eq!(c.frames.len(), aspects.len());
        if c.family == Family::Across {
            continue;
        }
        let rects: Vec<Rect> = c.frames.iter().map(|f| mm(spec, f)).collect();
        for (r, aspect) in rects.iter().zip(aspects) {
            let ratio = r.width.get() / r.height.get();
            assert!(
                (ratio / aspect - 1.0).abs() < 0.005,
                "proporção {ratio} ≠ {aspect}"
            );
            let on_left = inside(r, &spec.safe_rect(PageSide::Left));
            let on_right = inside(r, &spec.safe_rect(PageSide::Right));
            assert!(on_left || on_right, "fora da área útil: {r:?}");
            assert!(
                (on_left && active.left) || (on_right && active.right),
                "página inativa"
            );
        }
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                let apart_x = a.right().get() + gap <= b.x.get() + 1e-6
                    || b.right().get() + gap <= a.x.get() + 1e-6;
                let apart_y = a.bottom().get() + gap <= b.y.get() + 1e-6
                    || b.bottom().get() + gap <= a.y.get() + 1e-6;
                assert!(
                    apart_x || apart_y,
                    "quadros colados ou sobrepostos: {a:?} {b:?}"
                );
            }
        }
    }
}

#[test]
fn tp_b3_proporcao_preservada_em_todos_os_candidatos() {
    let s = spec(DesignMode::Spread);
    let mixes: [&[f64]; 6] = [
        &[V],
        &[H, V],
        &[V, Q, H],
        &[H, H, V, V],
        &[Q, Q, Q, Q],
        &[V, V, V],
    ];
    for aspects in mixes {
        let all = candidates(DesignMode::Spread, RIGHT, aspects);
        assert!(!all.is_empty());
        assert_valid(&s, RIGHT, aspects, &all);
    }
}

#[test]
fn tp_b4_seis_fotos_divididas_entre_as_paginas() {
    let s = spec(DesignMode::Spread);
    let aspects = [H, H, V, H, V, H];
    let all = candidates(DesignMode::Spread, BOTH, &aspects);
    let splits: Vec<(usize, usize)> = all
        .iter()
        .filter_map(|c| match &c.family {
            Family::PerPage {
                left: Some(l),
                right: Some(r),
            } => Some((l.groups.iter().sum(), r.groups.iter().sum())),
            _ => None,
        })
        .collect();
    for split in [(2, 4), (3, 3), (4, 2)] {
        assert!(splits.contains(&split), "falta a divisão {split:?}");
    }
    for c in &all {
        for f in &c.frames {
            let r = mm(&s, f);
            assert!(
                r.right().get() <= 300.0 + 1e-6 || r.x.get() >= 300.0 - 1e-6,
                "atravessa: {r:?}"
            );
        }
    }
    assert_valid(&s, BOTH, &aspects, &all);
}

#[test]
fn tp_b5_foto_atravessada_so_no_modo_lamina() {
    let spread = candidates(DesignMode::Spread, BOTH, &[H]);
    let across = spread
        .iter()
        .find(|c| c.family == Family::Across)
        .expect("atravessada");
    assert_eq!(across.frames, [RelRect::full()]);
    let page = candidates(DesignMode::Page, BOTH, &[H]);
    assert!(page.iter().all(|c| c.family != Family::Across));
}

#[test]
fn tp_b6_ponta_com_pagina_simples() {
    let s = spec(DesignMode::Spread);
    let all = candidates(DesignMode::Spread, RIGHT, &[H, V, H]);
    for c in &all {
        for f in &c.frames {
            assert!(mm(&s, f).x.get() >= 300.0, "quadro fora da página direita");
        }
    }
}

#[test]
fn tp_b7_muitas_fotos_para_as_paginas() {
    // Limite ampliado de 4 para 12 fotos por página pela spec VL (regra 1).
    let err =
        layout_candidates(&spec(DesignMode::Spread), BOTH, &[H; 25], DEFAULT_GAP).unwrap_err();
    assert_eq!(
        err,
        LayoutError::TooManyPhotos {
            photos: 25,
            pages: 2,
            capacity: 24
        }
    );
    assert_eq!(
        err.to_string(),
        "São 25 fotos para 2 páginas: cabem no máximo 24."
    );
}

#[test]
fn tp_b8_ordem_pelo_aproveitamento() {
    let all = candidates(DesignMode::Spread, RIGHT, &[H, H, H, H]);
    assert!(all.len() > 1);
    for pair in all.windows(2) {
        assert!(pair[0].coverage + 1e-9 >= pair[1].coverage, "fora de ordem");
    }
    assert_eq!(all, candidates(DesignMode::Spread, RIGHT, &[H, H, H, H]));
}

#[test]
fn tp_b9_uma_foto_na_lamina_vai_para_a_pagina_direita() {
    let s = spec(DesignMode::Page);
    let all = candidates(DesignMode::Page, BOTH, &[V]);
    assert_eq!(all.len(), 1, "{all:?}");
    assert!(mm(&s, &all[0].frames[0]).x.get() >= 300.0);
    assert!(matches!(all[0].family, Family::PerPage { left: None, .. }));
}

#[test]
fn erros_de_entrada() {
    let s = spec(DesignMode::Spread);
    assert_eq!(
        layout_candidates(&s, BOTH, &[], DEFAULT_GAP),
        Err(LayoutError::NoPhotos)
    );
    assert_eq!(
        layout_candidates(&s, BOTH, &[H, 0.0], DEFAULT_GAP),
        Err(LayoutError::InvalidAspect(2))
    );
    assert_eq!(
        layout_candidates(&s, RIGHT, &[H, H], Mm(500.0)),
        Err(LayoutError::GapTooLarge)
    );
}

/// Regras 1, 2 e 5 para muitas combinações de fotos, modos e pontas.
#[test]
fn integridade_com_combinacoes_variadas() {
    let kinds = [H, V, Q, 16.0 / 9.0, 4.0 / 5.0];
    let mut seed: u64 = 7;
    let mut next = |max: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        usize::try_from((seed >> 33) % max).unwrap()
    };
    for _ in 0..150 {
        let mode = if next(2) == 0 {
            DesignMode::Spread
        } else {
            DesignMode::Page
        };
        let active = [
            BOTH,
            RIGHT,
            ActivePages {
                left: true,
                right: false,
            },
        ][next(3)];
        let pages = usize::from(active.left) + usize::from(active.right);
        let count = next(u64::try_from(4 * pages).unwrap()) + 1;
        let aspects: Vec<f64> = (0..count).map(|_| kinds[next(5)]).collect();
        let s = spec(mode);
        let all = layout_candidates(&s, active, &aspects, DEFAULT_GAP).unwrap();
        assert!(!all.is_empty());
        assert_valid(&s, active, &aspects, &all);
    }
}
