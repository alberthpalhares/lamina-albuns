//! Cenários AB-B1 a AB-B6 da spec `specs/auto-build.md`. Os demais ficam em `tests/more.rs`.

use std::collections::HashSet;

use lamina_core::{AlbumEdge, AlbumSpec, CaptureTime, DesignMode, Dpi, Mm, Project};

use super::*;
use crate::layouts::FamilyKind;

/// Álbum 30 × 30 cm, zona segura de 10 mm, modo lâmina.
pub fn spec() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
        .with_mode(DesignMode::Spread)
}

/// Momentos de `per` fotos, 40 s entre fotos e 1 h entre momentos; uma vertical a
/// cada 4 fotos. A ordem de entrada segue `order` (índices das fotos).
pub fn album(spec: AlbumSpec, per: usize, order: &[usize]) -> Project {
    let mut project = Project::new("teste", spec);
    for &i in order {
        let (m, k) = (i / per, i % per);
        let (w, h) = if i % 4 == 3 {
            (2000, 3000)
        } else {
            (3000, 2000)
        };
        project.add_photo(format!("f{i:03}.jpg"), w, h);
        let seconds = i64::try_from(m * 3600 + k * 40).unwrap();
        let base = CaptureTime::from_exif("2026:09:12 15:00:00")
            .unwrap()
            .seconds();
        project.photos.last_mut().unwrap().taken_at =
            Some(CaptureTime::from_seconds(base + seconds));
    }
    project
}

pub fn forty() -> Project {
    album(spec(), 8, &(0..40).collect::<Vec<_>>())
}

pub fn build(project: &Project, spreads: usize, reuse: Reuse) -> AutoBuild {
    let options = BuildOptions {
        reuse,
        ..BuildOptions::default()
    };
    auto_build(project, spreads, &options).unwrap()
}

/// Nome do arquivo de cada quadro, lâmina por lâmina.
pub fn names(project: &Project) -> Vec<Vec<String>> {
    project
        .spreads
        .iter()
        .map(|s| {
            s.frames
                .iter()
                .map(|f| match &f.content {
                    FrameContent::Photo { photo, .. } => {
                        project.photo(*photo).unwrap().path.display().to_string()
                    }
                    other => panic!("quadro sem foto: {other:?}"),
                })
                .collect()
        })
        .collect()
}

#[test]
fn ab_b1_album_completo() {
    let result = build(&forty(), 10, Reuse::Medium);
    assert_eq!(result.project.spreads.len(), 10);
    let all: Vec<String> = names(&result.project).into_iter().flatten().collect();
    let unique: HashSet<&String> = all.iter().collect();
    assert_eq!((all.len(), unique.len()), (40, 40), "cada foto uma vez");
    let mut sorted = all.clone();
    sorted.sort();
    assert_eq!(all, sorted, "fora da ordem de captura");
}

#[test]
fn ab_b2_destaque_ganha_foto_grande() {
    let result = build(&forty(), 10, Reuse::Medium);
    let highlights: Vec<&SpreadReport> = result
        .report
        .iter()
        .filter(|r| r.role == SpreadRole::Highlight)
        .collect();
    assert!(!highlights.is_empty());
    for r in highlights {
        assert!(
            matches!(
                r.family.kind(),
                FamilyKind::Across
                    | FamilyKind::BleedPage
                    | FamilyKind::Hero
                    | FamilyKind::WholeSpread
            ),
            "destaque com {:?}",
            r.family
        );
    }
}

fn distinct(report: &[SpreadReport]) -> usize {
    let mut seen: Vec<&Family> = Vec::new();
    for r in report {
        if !seen.iter().any(|f| f.same_outline(&r.family)) {
            seen.push(&r.family);
        }
    }
    seen.len()
}

#[test]
fn ab_b3_nivel_de_reutilizacao_muda_a_variedade() {
    // Um momento de 24 fotos, todas horizontais.
    let project = album(spec(), 24, &(0..24).collect::<Vec<_>>());
    let mut horizontals = project;
    for p in &mut horizontals.photos {
        (p.width_px, p.height_px) = (3000, 2000);
    }
    let counts: Vec<usize> = [Reuse::Low, Reuse::Medium, Reuse::High]
        .into_iter()
        .map(|reuse| {
            let r = build(&horizontals, 8, reuse).report;
            if reuse != Reuse::High {
                for pair in r.windows(2) {
                    assert!(
                        !pair[0].family.same_outline(&pair[1].family),
                        "{reuse:?}: vizinhas iguais"
                    );
                }
            }
            distinct(&r)
        })
        .collect();
    assert!(
        counts[0] >= counts[1] && counts[1] >= counts[2],
        "{counts:?}"
    );
    assert!(counts[0] > counts[2], "o nível não mudou nada: {counts:?}");
}

#[test]
fn ab_b4_foto_inteira_quando_nao_ha_recorte() {
    let project = forty();
    let result = build(&project, 10, Reuse::Medium);
    let trim = result.project.spec.trim_rect();
    for (spread, r) in result.project.spreads.iter().zip(&result.report) {
        let crops = matches!(
            r.family.kind(),
            FamilyKind::Across | FamilyKind::BleedPage | FamilyKind::Mosaic
        );
        for f in &spread.frames {
            let FrameContent::Photo { photo, .. } = f.content else {
                panic!("quadro sem foto");
            };
            let rect = f.rect.to_absolute(&trim);
            let (frame, photo) = (
                rect.width.get() / rect.height.get(),
                project.photo(photo).unwrap().aspect_ratio(),
            );
            let visible = (frame / photo).min(photo / frame);
            if crops {
                // Limite de cada família (VL, regras 2 e 5): mosaico 15%, as outras 35%
                // (auditoria do Marco 1, M10: antes o mosaico podia passar com 35%).
                let limit = if r.family.kind() == FamilyKind::Mosaic {
                    0.85
                } else {
                    0.65
                };
                assert!(
                    visible >= limit - 1e-9,
                    "{:?}: recorte acima do limite",
                    r.family
                );
            } else {
                assert!(
                    (frame / photo - 1.0).abs() < 0.005,
                    "{:?}: proporção mudou",
                    r.family
                );
            }
        }
    }
}

#[test]
fn ab_b5_pagina_simples_nas_pontas() {
    let spec = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let project = album(spec, 14, &(0..14).collect::<Vec<_>>());
    let result = build(&project, 3, Reuse::Medium);
    let spreads = &result.project.spreads;
    assert!(
        spreads[0].frames.iter().all(|f| f.rect.x() >= 0.5 - 1e-9),
        "primeira fora da direita"
    );
    assert!(
        spreads[2]
            .frames
            .iter()
            .all(|f| f.rect.x() + f.rect.width() <= 0.5 + 1e-9),
        "última fora da esquerda"
    );
}

#[test]
fn ab_b6_projeto_valido_e_salvavel() {
    let result = build(&forty(), 10, Reuse::Medium);
    result.project.validate().unwrap();
    let json = result.project.to_json().unwrap();
    assert_eq!(Project::from_json(&json).unwrap(), result.project);
}
