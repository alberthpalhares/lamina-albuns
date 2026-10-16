//! Cenários AB-B7 a AB-B11 da spec `specs/auto-build.md` e o teste de integridade.

use std::collections::HashSet;
use std::time::Instant;

use lamina_core::{AlbumEdge, Project};

use super::tests::{album, build, forty, names, spec};
use super::*;
use lamina_core::testing::budget;

#[test]
fn ab_b7_foto_sem_tamanho() {
    let mut project = Project::new("teste", spec());
    for (name, w) in [("a.jpg", 3000), ("b.jpg", 0), ("c.jpg", 3000)] {
        project.add_photo(name, w, 2000);
    }
    let err = auto_build(&project, 1, &BuildOptions::default()).unwrap_err();
    assert!(matches!(&err, BuildError::InvalidPhoto(name) if name == "b.jpg"));
    assert_eq!(err.to_string(), "A foto b.jpg não tem tamanho válido.");
}

#[test]
fn ab_b8_erros_do_plano_passam_adiante() {
    let project = album(spec(), 100, &(0..100).collect::<Vec<_>>());
    let err = auto_build(&project, 10, &BuildOptions::default()).unwrap_err();
    assert!(matches!(
        err,
        BuildError::Plan(PlanError::TooManyPhotos {
            photos: 100,
            capacity: 80,
            spreads: 10,
            min_spreads: 13
        })
    ));
    assert_eq!(
        err.to_string(),
        "São 100 fotos e cabem 80 em 10 lâminas: use pelo menos 13 lâminas."
    );
}

#[test]
fn ab_b9_mesma_entrada_mesmo_album() {
    let reference = build(&forty(), 10, Reuse::Medium);
    let shuffled: Vec<usize> = (0..40).map(|i| (i * 17 + 5) % 40).collect();
    let other = build(&album(spec(), 8, &shuffled), 10, Reuse::Medium);
    assert_eq!(names(&other.project), names(&reference.project));
    let rects = |p: &Project| -> Vec<Vec<lamina_core::RelRect>> {
        p.spreads
            .iter()
            .map(|s| s.frames.iter().map(|f| f.rect).collect())
            .collect()
    };
    assert_eq!(rects(&other.project), rects(&reference.project));
    assert_eq!(other.report, reference.report);
}

#[test]
fn ab_b10_o_relatorio_explica_cada_escolha() {
    let result = build(&forty(), 10, Reuse::Medium);
    assert_eq!(result.report.len(), 10);
    for (r, spread) in result.report.iter().zip(&result.project.spreads) {
        assert_eq!(r.photos, spread.frames.len());
        assert!((1..=12).contains(&r.position), "posição {}", r.position);
        assert!((0.0..=100.0).contains(&r.score));
        assert!(!r.reason.is_empty());
    }
    let moments: HashSet<usize> = result.report.iter().map(|r| r.moment).collect();
    assert_eq!(moments.len(), 5);
    assert!(
        result
            .report
            .iter()
            .any(|r| r.reason.contains("destaque: foto grande"))
    );
}

#[test]
fn ab_b11_rapido_para_albuns_grandes() {
    let project = album(spec(), 20, &(0..200).collect::<Vec<_>>());
    let fastest = (0..2)
        .map(|_| {
            let start = Instant::now();
            build(&project, 50, Reuse::Medium);
            start.elapsed()
        })
        .min()
        .unwrap();
    assert!(fastest < budget(2000), "{fastest:?}");
}

/// Para álbuns variados: toda foto uma vez, em ordem, projeto válido, relatório completo.
#[test]
fn integridade_do_auto_build() {
    let edges = [AlbumEdge::FullSpread, AlbumEdge::SinglePage];
    let mut seed: u64 = 3;
    let mut next = |max: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        usize::try_from((seed >> 33) % max).unwrap()
    };
    let mut built = 0;
    for _ in 0..12 {
        let s = spec().with_edges(edges[next(2)], edges[next(2)]);
        let (moments, per) = (next(4) + 1, next(10) + 3);
        let total = moments * per;
        let project = album(s, per, &(0..total).collect::<Vec<_>>());
        let spreads = (total / 4).max(1) + next(3);
        let reuse = [Reuse::Low, Reuse::Medium, Reuse::High][next(3)];
        let options = BuildOptions {
            reuse,
            ..BuildOptions::default()
        };
        let result = match auto_build(&project, spreads.min(total), &options) {
            Ok(result) => result,
            // Só o plano pode recusar (lâminas de menos para as fotos); o resto é defeito.
            Err(err) => {
                assert!(matches!(err, BuildError::Plan(_)), "{err}");
                continue;
            }
        };
        built += 1;
        result.project.validate().unwrap();
        let all: Vec<String> = names(&result.project).into_iter().flatten().collect();
        let mut sorted = all.clone();
        sorted.sort();
        assert_eq!(all.len(), total);
        assert_eq!(all, sorted);
        assert_eq!(result.report.len(), result.project.spreads.len());
    }
    assert!(built >= 8, "só {built} de 12 álbuns montados");
}
