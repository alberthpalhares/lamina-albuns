//! Ordens das sugestões reordenadas (spec VL, regra 7).

use super::families::orientation;

/// Até 6 ordens diferentes da original, cada uma mantendo a ordem dentro de cada orientação.
/// Vazia se as fotos não misturam verticais com as demais.
pub fn reorderings(aspects: &[f64]) -> Vec<Vec<usize>> {
    let (verticals, others): (Vec<usize>, Vec<usize>) =
        (0..aspects.len()).partition(|&i| orientation(aspects[i]) < 0);
    if verticals.is_empty() || others.is_empty() {
        return Vec::new();
    }
    let (vs, hs) = (&verticals[..], &others[..]);
    let (v_half, h_half) = (vs.len().div_ceil(2), hs.len().div_ceil(2));
    let candidates = [
        [hs, vs].concat(),
        [vs, hs].concat(),
        [&vs[..v_half], hs, &vs[v_half..]].concat(),
        [&hs[..h_half], vs, &hs[h_half..]].concat(),
        alternate(hs, vs),
        alternate(vs, hs),
    ];
    let original: Vec<usize> = (0..aspects.len()).collect();
    let mut out: Vec<Vec<usize>> = Vec::new();
    for order in candidates {
        if order != original && !out.contains(&order) {
            out.push(order);
        }
    }
    out
}

/// Intercala começando por `first`; o que sobrar vai no fim, na ordem.
fn alternate(first: &[usize], second: &[usize]) -> Vec<usize> {
    let mut out = Vec::with_capacity(first.len() + second.len());
    for i in 0..first.len().max(second.len()) {
        out.extend(first.get(i));
        out.extend(second.get(i));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: f64 = 1.5;
    const V: f64 = 2.0 / 3.0;

    #[test]
    fn so_uma_orientacao_nao_reordena() {
        assert!(reorderings(&[H, H, H]).is_empty());
    }

    #[test]
    fn duas_horizontais_e_duas_verticais() {
        // Fotos 0 e 1 horizontais, 2 e 3 verticais: a original já é "horizontais primeiro".
        let all = reorderings(&[H, H, V, V]);
        assert_eq!(
            all,
            [
                vec![2, 3, 0, 1],
                vec![2, 0, 1, 3],
                vec![0, 2, 3, 1],
                vec![0, 2, 1, 3],
                vec![2, 0, 3, 1],
            ]
        );
    }
}
