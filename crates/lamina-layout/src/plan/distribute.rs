//! Regra 5 da spec: ritmo dentro do momento. A primeira lâmina é o destaque (2 fotos, ou 1 se
//! não sobrar para as outras) e as seguintes dividem o resto por igual, com a sobra nas
//! primeiras e respeitando a capacidade de cada uma.

/// Fotos de destaque quando o momento tem mais de uma lâmina.
const HIGHLIGHT_PHOTOS: usize = 2;

/// Quantas fotos vão em cada lâmina do momento. `caps` são as capacidades das lâminas dele, em
/// ordem; o chamador garante `caps.len() <= photos <= soma(caps)`.
pub fn distribute(photos: usize, caps: &[usize]) -> Vec<usize> {
    let Some((first_cap, dense_caps)) = caps.split_first() else {
        return Vec::new();
    };
    if dense_caps.is_empty() {
        return vec![photos];
    }
    let dense_room: usize = dense_caps.iter().sum();
    let mut highlight = HIGHLIGHT_PHOTOS.min(photos - dense_caps.len()).max(1);
    if photos - highlight > dense_room {
        highlight = (photos - dense_room).min(*first_cap);
    }
    let mut counts = vec![highlight];
    counts.extend(fill_evenly(photos - highlight, dense_caps));
    counts
}

/// Distribui `photos` entre lâminas com capacidades `caps`, uma por vez para a lâmina com menos
/// fotos que ainda tem espaço (empate: a primeira). Cada lâmina começa com 1.
fn fill_evenly(photos: usize, caps: &[usize]) -> Vec<usize> {
    let mut counts = vec![1; caps.len()];
    for _ in caps.len()..photos {
        let next = (0..caps.len())
            .filter(|&i| counts[i] < caps[i])
            .min_by_key(|&i| (counts[i], i));
        match next {
            Some(i) => counts[i] += 1,
            None => break,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uma_lamina_leva_todas() {
        assert_eq!(distribute(6, &[8]), [6]);
    }

    #[test]
    fn destaque_com_duas_e_resto_por_igual() {
        assert_eq!(distribute(8, &[8, 8]), [2, 6]);
        assert_eq!(distribute(9, &[8, 8, 8]), [2, 4, 3]);
        assert_eq!(distribute(30, &[8; 5]), [2, 7, 7, 7, 7]);
    }

    #[test]
    fn destaque_com_uma_quando_nao_sobra() {
        assert_eq!(distribute(2, &[8, 8]), [1, 1]);
        assert_eq!(distribute(3, &[8, 8, 8]), [1, 1, 1]);
    }

    #[test]
    fn respeita_a_capacidade_das_pontas() {
        assert_eq!(distribute(14, &[4, 8, 4]), [2, 8, 4]);
    }

    #[test]
    fn destaque_cresce_quando_as_densas_estao_cheias() {
        assert_eq!(distribute(12, &[8, 8]), [4, 8]);
    }
}
