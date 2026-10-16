//! Cenários da spec `specs/plano-de-laminas.md` (PL-B1 a PL-B9) e o teste de integridade.

use lamina_core::{AlbumEdge, AlbumSpec, CaptureTime, Dpi, Edges, Mm, Photo, PhotoId, Project};

use super::*;

/// Fotos de teste: (nome do arquivo, data no formato EXIF ou `None`).
fn photos(list: &[(&str, Option<&str>)]) -> Vec<Photo> {
    let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
    let mut project = Project::new("teste", spec);
    for (name, taken) in list {
        project.add_photo(*name, 3000, 2000);
        let photo = project.photos.last_mut().unwrap();
        photo.taken_at = taken.map(|t| CaptureTime::from_exif(t).unwrap());
    }
    project.photos
}

/// `count` fotos a partir de `start` (EXIF), uma a cada `step` segundos, com nomes `prefixo_NN.jpg`.
fn burst(prefix: &str, start: &str, count: usize, step: i64) -> Vec<(String, Option<String>)> {
    let t0 = CaptureTime::from_exif(start).unwrap().seconds();
    (0..count)
        .map(|i| {
            let t = CaptureTime::from_seconds(t0 + step * i64::try_from(i).unwrap());
            let exif = t.to_string().replace('-', ":");
            (format!("{prefix}_{i:02}.jpg"), Some(exif))
        })
        .collect()
}

fn as_refs(list: &[(String, Option<String>)]) -> Vec<(&str, Option<&str>)> {
    list.iter()
        .map(|(n, t)| (n.as_str(), t.as_deref()))
        .collect()
}

/// As 40 fotos do PL-B1: 5 momentos de 8 fotos, 40 s entre fotos, 1 h entre momentos.
fn forty() -> Vec<(String, Option<String>)> {
    (0..5)
        .flat_map(|m| {
            burst(
                &format!("m{m}"),
                &format!("2026:09:12 {:02}:00:00", 15 + m),
                8,
                40,
            )
        })
        .collect()
}

fn names(all: &[Photo], plan: &SpreadPlan) -> Vec<String> {
    plan.photos
        .iter()
        .map(|id| {
            let p = all.iter().find(|p| p.id == *id).unwrap();
            p.path.to_string_lossy().into_owned()
        })
        .collect()
}

fn sizes(plan: &[SpreadPlan]) -> Vec<usize> {
    plan.iter().map(|s| s.photos.len()).collect()
}

fn plan(list: &[Photo], spreads: usize) -> Result<Vec<SpreadPlan>, PlanError> {
    plan_spreads(list, spreads, Edges::default(), &PlanOptions::default())
}

#[test]
fn pl_b1_quarenta_fotos_em_cinco_momentos() {
    let list = photos(&as_refs(&forty()));
    let result = plan(&list, 10).unwrap();
    assert_eq!(sizes(&result), [2, 6, 2, 6, 2, 6, 2, 6, 2, 6]);
    for (i, spread) in result.iter().enumerate() {
        assert_eq!(spread.moment, i / 2);
        let role = if i % 2 == 0 {
            SpreadRole::Highlight
        } else {
            SpreadRole::Dense
        };
        assert_eq!(spread.role, role);
    }
    assert_eq!(names(&list, &result[0]), ["m0_00.jpg", "m0_01.jpg"]);
    let all: Vec<PhotoId> = result.iter().flat_map(|s| s.photos.clone()).collect();
    let expected: Vec<PhotoId> = list.iter().map(|p| p.id).collect();
    assert_eq!(all, expected);
}

#[test]
fn pl_b2_fotos_sem_data_vao_para_o_fim() {
    let mut list = burst("d", "2026:09:12 15:00:00", 6, 40);
    for name in ["c.jpg", "a.jpg", "b.jpg"] {
        list.push((name.to_owned(), None));
    }
    let list = photos(&as_refs(&list));
    let result = plan(&list, 2).unwrap();
    assert_eq!(names(&list, &result[1]), ["a.jpg", "b.jpg", "c.jpg"]);
    assert_eq!(result[1].role, SpreadRole::Single);
}

#[test]
fn pl_b3_momentos_demais_sao_unidos_pelos_vizinhos_mais_proximos() {
    let mut list = burst("a", "2026:09:12 15:00:00", 3, 20);
    list.extend(burst("b", "2026:09:12 15:30:00", 3, 20));
    list.extend(burst("c", "2026:09:12 18:30:00", 3, 20));
    let list = photos(&as_refs(&list));
    let result = plan(&list, 2).unwrap();
    assert_eq!(sizes(&result), [6, 3]);
    assert_eq!(
        names(&list, &result[1]),
        ["c_00.jpg", "c_01.jpg", "c_02.jpg"]
    );
    assert_eq!(result[0].moment, 0);
    assert_eq!(result[1].moment, 1);
}

#[test]
fn pl_b4_laminas_em_proporcao_ao_tamanho_do_momento() {
    let mut list = burst("a", "2026:09:12 15:00:00", 30, 30);
    list.extend(burst("b", "2026:09:12 18:00:00", 6, 30));
    let list = photos(&as_refs(&list));
    let result = plan(&list, 6).unwrap();
    assert_eq!(sizes(&result), [2, 7, 7, 7, 7, 6]);
    let moments: Vec<usize> = result.iter().map(|s| s.moment).collect();
    assert_eq!(moments, [0, 0, 0, 0, 0, 1]);
    assert_eq!(result[5].role, SpreadRole::Single);
}

#[test]
fn pl_b5_fotos_demais_para_as_laminas_pedidas() {
    let list = photos(&as_refs(&burst("f", "2026:09:12 15:00:00", 100, 10)));
    let err = plan(&list, 10).unwrap_err();
    assert_eq!(
        err.to_string(),
        "São 100 fotos e cabem 80 em 10 lâminas: use pelo menos 13 lâminas."
    );
}

#[test]
fn pl_b6_laminas_demais_para_as_fotos() {
    let list = photos(&as_refs(&burst("f", "2026:09:12 15:00:00", 12, 10)));
    let err = plan(&list, 20).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Há 12 fotos para 20 lâminas: use no máximo 12 lâminas."
    );
}

#[test]
fn pl_b7_rajada_no_mesmo_segundo() {
    let t = Some("2026:09:12 15:00:00");
    let list = photos(&[
        ("e.jpg", t),
        ("b.jpg", t),
        ("d.jpg", t),
        ("a.jpg", t),
        ("c.jpg", t),
    ]);
    let result = plan(&list, 1).unwrap();
    assert_eq!(
        names(&list, &result[0]),
        ["a.jpg", "b.jpg", "c.jpg", "d.jpg", "e.jpg"]
    );
    assert_eq!((result[0].moment, result[0].role), (0, SpreadRole::Single));
}

#[test]
fn pl_b8_pagina_simples_nas_pontas_reduz_a_capacidade() {
    let list = photos(&as_refs(&burst("f", "2026:09:12 15:00:00", 14, 30)));
    let edges = Edges::new(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let result = plan_spreads(&list, 3, edges, &PlanOptions::default()).unwrap();
    assert_eq!(sizes(&result), [2, 8, 4]);
}

#[test]
fn pl_b9_mesma_entrada_mesmo_plano() {
    let ordered = photos(&as_refs(&forty()));
    let mut shuffled = ordered.clone();
    // Embaralhamento fixo: troca posições de forma determinística.
    for i in 0..shuffled.len() {
        let j = (i * 17 + 5) % shuffled.len();
        shuffled.swap(i, j);
    }
    assert_ne!(ordered, shuffled);
    assert_eq!(plan(&ordered, 10).unwrap(), plan(&shuffled, 10).unwrap());
}

/// Regra 6: para entradas variadas, toda foto aparece uma vez, a ordem se mantém e nenhuma
/// lâmina passa da capacidade.
#[test]
fn integridade_com_entradas_variadas() {
    let mut seed: u64 = 42;
    let mut next = |max: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 33) % max
    };
    for _ in 0..200 {
        let count = usize::try_from(next(60) + 1).unwrap();
        let mut list = Vec::new();
        let mut t = CaptureTime::from_exif("2026:09:12 10:00:00")
            .unwrap()
            .seconds();
        for i in 0..count {
            t += i64::try_from(next(3600)).unwrap();
            let exif =
                (next(10) > 0).then(|| CaptureTime::from_seconds(t).to_string().replace('-', ":"));
            list.push((format!("f{i:03}.jpg"), exif));
        }
        let list = photos(&as_refs(&list));
        let edges = Edges::new(
            if next(2) == 0 {
                AlbumEdge::FullSpread
            } else {
                AlbumEdge::SinglePage
            },
            if next(2) == 0 {
                AlbumEdge::FullSpread
            } else {
                AlbumEdge::SinglePage
            },
        );
        let spreads = usize::try_from(next(u64::try_from(count).unwrap()) + 1).unwrap();
        let Ok(result) = plan_spreads(&list, spreads, edges, &PlanOptions::default()) else {
            continue;
        };
        assert_eq!(result.len(), spreads);
        let mut seen: Vec<PhotoId> = result.iter().flat_map(|s| s.photos.clone()).collect();
        let in_order = seen.clone();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), count, "foto repetida ou perdida");
        let mut expected = in_order.clone();
        expected.sort_by_key(|id| {
            let p = list.iter().find(|p| p.id == *id).unwrap();
            (p.taken_at.is_none(), p.taken_at, p.path.clone())
        });
        assert_eq!(in_order, expected, "ordem cronológica quebrada");
        for (i, s) in result.iter().enumerate() {
            let pages = edges.active_pages(i, spreads);
            let cap = 4 * (usize::from(pages.left) + usize::from(pages.right)).max(1);
            assert!(
                !s.photos.is_empty() && s.photos.len() <= cap,
                "lâmina {i} fora da capacidade"
            );
        }
    }
}

#[test]
fn fotos_por_pagina_fora_de_1_a_12_e_erro() {
    let list = photos(&as_refs(&forty()));
    for value in [0, 13] {
        let options = PlanOptions {
            max_per_page: value,
            ..PlanOptions::default()
        };
        let err = plan_spreads(&list, 10, Edges::default(), &options).unwrap_err();
        assert_eq!(err, PlanError::InvalidMaxPerPage { value, max: 12 });
    }
    let err = PlanError::InvalidMaxPerPage { value: 0, max: 12 };
    assert_eq!(err.to_string(), "Fotos por página: use de 1 a 12, não 0.");
    let twelve = PlanOptions {
        max_per_page: 12,
        ..PlanOptions::default()
    };
    // Com 12 por página, 40 fotos cabem em 4 lâminas (capacidade 96).
    let result = plan_spreads(&list, 4, Edges::default(), &twelve).unwrap();
    assert_eq!(result.iter().map(|s| s.photos.len()).sum::<usize>(), 40);
}
