//! Teste de integridade da spec VL: área útil, proporção, espaço mínimo, recorte nos limites,
//! lombada e nota, para muitas combinações de fotos, modos e pontas.

use lamina_core::{ActivePages, AlbumSpec, DesignMode, Dpi, Mm, PageSide, Rect};

use super::*;

fn spec(mode: DesignMode) -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
        .with_mode(mode)
}

fn inside(r: &Rect, area: &Rect) -> bool {
    r.x.get() >= area.x.get() - 1e-6
        && r.y.get() >= area.y.get() - 1e-6
        && r.right().get() <= area.right().get() + 1e-6
        && r.bottom().get() <= area.bottom().get() + 1e-6
}

fn check(s: &AlbumSpec, active: ActivePages, aspects: &[f64], c: &LayoutCandidate) {
    let trim = s.trim_rect();
    let rects: Vec<Rect> = c.frames.iter().map(|f| f.to_absolute(&trim)).collect();
    assert_eq!(rects.len(), aspects.len());
    assert!((0.0..=100.0).contains(&c.score.total), "{:?}", c.score);
    let (left, right) = (s.safe_rect(PageSide::Left), s.safe_rect(PageSide::Right));
    let spread = Rect::new(left.x, left.y, right.right() - left.x, left.height);
    let crops = matches!(
        c.family.kind(),
        FamilyKind::Across | FamilyKind::BleedPage | FamilyKind::Mosaic
    );
    for (i, (r, aspect)) in rects.iter().zip(aspects).enumerate() {
        if crops {
            assert!(c.crop[i] <= 0.35 + 1e-9);
            continue;
        }
        let ratio = r.width.get() / r.height.get();
        assert!(
            (ratio / aspect - 1.0).abs() < 0.005,
            "proporção {ratio} ≠ {aspect}"
        );
        let on_left = active.left && inside(r, &left);
        let on_right = active.right && inside(r, &right);
        let both = active.left && active.right && s.mode() == DesignMode::Spread;
        assert!(
            on_left || on_right || (both && inside(r, &spread)),
            "fora da área útil: {r:?}"
        );
    }
    let gap = DEFAULT_GAP.get();
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            let apart_x = a.right().get() + gap <= b.x.get() + 1e-6
                || b.right().get() + gap <= a.x.get() + 1e-6;
            let apart_y = a.bottom().get() + gap <= b.y.get() + 1e-6
                || b.bottom().get() + gap <= a.y.get() + 1e-6;
            assert!(
                apart_x || apart_y,
                "{:?}: quadros colados {a:?} {b:?}",
                c.family
            );
        }
    }
}

#[test]
fn integridade_da_variedade_com_combinacoes_variadas() {
    let kinds = [1.5, 2.0 / 3.0, 1.0, 16.0 / 9.0, 4.0 / 5.0];
    let mut seed: u64 = 11;
    let mut next = |max: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        usize::try_from((seed >> 33) % max).unwrap()
    };
    let sides = [
        ActivePages {
            left: true,
            right: true,
        },
        ActivePages {
            left: false,
            right: true,
        },
        ActivePages {
            left: true,
            right: false,
        },
    ];
    for _ in 0..40 {
        let mode = [DesignMode::Spread, DesignMode::Page][next(2)];
        let active = sides[next(3)];
        let pages = usize::from(active.left) + usize::from(active.right);
        let count = next(u64::try_from(MAX_PER_PAGE * pages).unwrap()) + 1;
        let aspects: Vec<f64> = (0..count).map(|_| kinds[next(5)]).collect();
        let s = spec(mode);
        let options = LayoutOptions {
            max_candidates: 60,
            ..LayoutOptions::default()
        };
        let l = layout_options(&s, active, &aspects, &options).unwrap();
        assert!(!l.in_order.is_empty() && l.in_order.len() <= 60);
        assert!(l.reordered.len() <= 60);
        for c in l.in_order.iter().chain(&l.reordered) {
            check(&s, active, &aspects, c);
        }
    }
}

/// Achado M3 da auditoria do Marco 1: quadro largo que passa só 2 mm da lombada deixaria um
/// filete da foto na outra página.
#[test]
fn quadro_sobre_a_lombada_nao_deixa_filete() {
    use lamina_core::Mm;

    use super::area::Ctx;
    use super::rank::gutter_ok;

    let s = spec(DesignMode::Spread);
    let ctx = Ctx {
        spec: &s,
        active: ActivePages {
            left: true,
            right: true,
        },
        gap: 0.0,
        inset: 0.0,
    };
    // Lombada em 300 mm. Filete: 150 mm de largura, só 2 mm do lado direito.
    let thin = Rect::new(Mm(152.0), Mm(50.0), Mm(150.0), Mm(100.0));
    let centered = Rect::new(Mm(225.0), Mm(50.0), Mm(150.0), Mm(100.0));
    assert!(!gutter_ok(&ctx, &[thin]), "filete aceito");
    assert!(gutter_ok(&ctx, &[centered]));
    // Nenhuma sugestão da variedade deixa filete.
    let l = layout_options(&s, ctx.active, &[1.5; 8], &LayoutOptions::default()).unwrap();
    for c in l.in_order.iter().chain(&l.reordered) {
        for f in &c.frames {
            let (left, right) = (0.5 - f.x(), f.x() + f.width() - 0.5);
            if left > 1e-9 && right > 1e-9 {
                assert!(left >= 0.1 * f.width() - 1e-9 && right >= 0.1 * f.width() - 1e-9);
            }
        }
    }
}
