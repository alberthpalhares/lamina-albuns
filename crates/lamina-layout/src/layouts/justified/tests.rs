use lamina_core::{Mm, Rect};

use super::super::divisions::{MAX_BLOCK, MAX_GROUPS, compositions};
use super::*;

#[test]
fn composicoes_de_quatro() {
    let all = compositions(4);
    assert_eq!(all.len(), 8);
    assert_eq!(all[0], [4]);
    assert_eq!(&all[1..4], [vec![1, 3], vec![2, 2], vec![3, 1]]);
    assert_eq!(all[7], [1, 1, 1, 1]);
}

#[test]
fn uma_composicao_para_uma_foto() {
    assert_eq!(compositions(1), [vec![1]]);
    assert!(compositions(MAX_BLOCK + 1).is_empty());
    assert!(compositions(0).is_empty());
}

#[test]
fn acima_de_seis_fotos_so_divisoes_equilibradas() {
    let all = compositions(10);
    for groups in all {
        assert_eq!(groups.iter().sum::<usize>(), 10);
        let (min, max) = (groups.iter().min().unwrap(), groups.iter().max().unwrap());
        assert!(max - min <= 1, "desequilibrada: {groups:?}");
        assert!(groups.len() <= MAX_GROUPS);
    }
    for expected in [vec![4, 3, 3], vec![3, 4, 3], vec![3, 3, 4], vec![5, 5]] {
        assert!(all.contains(&expected), "falta {expected:?}");
    }
}

#[test]
fn coluna_e_a_linha_transposta() {
    let area = Rect::new(Mm(0.0), Mm(0.0), Mm(200.0), Mm(100.0));
    let column = PageLayout {
        arrangement: Arrangement::Columns,
        groups: vec![2],
    };
    let rects = arrange(area, &[1.0, 1.0], &column, 0.0).unwrap();
    // Duas quadradas empilhadas numa altura de 100: 50 × 50 cada, centralizadas.
    assert_eq!(rects[0], Rect::new(Mm(75.0), Mm(0.0), Mm(50.0), Mm(50.0)));
    assert_eq!(rects[1], Rect::new(Mm(75.0), Mm(50.0), Mm(50.0), Mm(50.0)));
}

#[test]
fn mosaico_enche_a_area_sem_sobra() {
    let area = Rect::new(Mm(0.0), Mm(0.0), Mm(200.0), Mm(110.0));
    let row = PageLayout {
        arrangement: Arrangement::Rows,
        groups: vec![2],
    };
    // Duas quadradas numa linha de 200: largura 100 cada, altura esticada para 110.
    let rects = mosaic(area, &[1.0, 1.0], &row, 0.0).unwrap();
    let expected = [(0.0, 0.0, 100.0, 110.0), (100.0, 0.0, 100.0, 110.0)];
    for (r, (x, y, w, h)) in rects.iter().zip(expected) {
        let got = [r.x.get(), r.y.get(), r.width.get(), r.height.get()];
        for (g, e) in got.into_iter().zip([x, y, w, h]) {
            assert!((g - e).abs() < 1e-9, "{r:?}");
        }
    }
}

#[test]
fn poda_mantem_os_arranjos_com_mais_area() {
    let area = Rect::new(Mm(0.0), Mm(0.0), Mm(280.0), Mm(280.0));
    let all = block_layouts(area, &[1.5; 5], 2.0, usize::MAX);
    let kept = block_layouts(area, &[1.5; 5], 2.0, 4);
    assert_eq!(kept.len(), 4);
    let worst_kept = kept
        .iter()
        .map(|(_, r)| covered(r))
        .fold(f64::MAX, f64::min);
    let better = all.iter().filter(|(_, r)| covered(r) > worst_kept + 1e-9);
    assert!(better.count() < 4, "ficou de fora um arranjo melhor");
}
