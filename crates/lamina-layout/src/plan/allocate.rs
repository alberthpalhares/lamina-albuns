//! Regra 4 da spec: quantas lâminas cada momento recebe, e a capacidade de cada lâmina.

use lamina_core::Edges;

/// Capacidade de cada uma das `spreads` lâminas: `per_page` × páginas ativas. Uma lâmina sem
/// página ativa (álbum de 1 lâmina com página simples nas duas pontas) conta como 1 página.
pub fn capacities(spreads: usize, edges: Edges, per_page: usize) -> Vec<usize> {
    (0..spreads)
        .map(|i| {
            let pages = edges.active_pages(i, spreads);
            per_page * (usize::from(pages.left) + usize::from(pages.right)).max(1)
        })
        .collect()
}

/// Menor número de lâminas em que cabem `photos` fotos.
pub fn min_spreads(photos: usize, edges: Edges, per_page: usize) -> usize {
    (1..)
        .find(|&s| capacities(s, edges, per_page).iter().sum::<usize>() >= photos)
        .unwrap_or(photos)
}

/// Lâminas por momento, em proporção ao tamanho (maiores restos), cada momento com pelo menos 1
/// e no máximo uma por foto. `Err(i)` quando o momento `i` não cabe nem tomando lâminas dos
/// outros.
pub fn allocate(sizes: &[usize], caps: &[usize]) -> Result<Vec<usize>, usize> {
    let spreads = caps.len();
    let total: usize = sizes.iter().sum();
    // Cota exata = tamanho × lâminas ÷ total. Trabalhamos com o numerador (inteiros) para não
    // depender de ponto flutuante: déficit_i = tamanho_i × lâminas − k_i × total.
    let deficit = |i: usize, k: usize| to_i128(sizes[i] * spreads) - to_i128(k * total);
    let mut k: Vec<usize> = sizes
        .iter()
        .map(|n| (n * spreads / total).clamp(1, *n))
        .collect();
    while k.iter().sum::<usize>() < spreads {
        let i = pick(&k, |i, ki| ki < sizes[i], |i, ki| -deficit(i, ki));
        k[i] += 1;
    }
    while k.iter().sum::<usize>() > spreads {
        let i = pick(&k, |_, ki| ki > 1, deficit);
        k[i] -= 1;
    }
    fix_capacity(sizes, caps, k)
}

/// Enquanto algum momento não couber, ele toma uma lâmina do momento com mais folga.
fn fix_capacity(sizes: &[usize], caps: &[usize], mut k: Vec<usize>) -> Result<Vec<usize>, usize> {
    for _ in 0..caps.len() * sizes.len() + 1 {
        let room = moment_capacities(&k, caps);
        let Some(i) = (0..sizes.len()).find(|&i| sizes[i] > room[i]) else {
            return Ok(k);
        };
        let donor = (0..sizes.len())
            .filter(|&j| j != i && k[j] > 1)
            .filter_map(|j| {
                let mut trial = k.clone();
                trial[j] -= 1;
                trial[i] += 1;
                let slack = moment_capacities(&trial, caps)[j].checked_sub(sizes[j])?;
                Some((slack, j))
            })
            .min_by_key(|&(slack, j)| (std::cmp::Reverse(slack), j));
        match donor {
            Some((_, j)) => {
                k[j] -= 1;
                k[i] += 1;
            }
            None => return Err(i),
        }
    }
    Err(0)
}

/// Capacidade somada das lâminas de cada momento, nas posições que elas ocupam no álbum.
pub fn moment_capacities(k: &[usize], caps: &[usize]) -> Vec<usize> {
    let mut start = 0;
    k.iter()
        .map(|&count| {
            let sum = caps[start..start + count].iter().sum();
            start += count;
            sum
        })
        .collect()
}

/// Índice que minimiza `key` entre os que passam em `allowed` (empate: o primeiro).
fn pick(
    k: &[usize],
    allowed: impl Fn(usize, usize) -> bool,
    key: impl Fn(usize, usize) -> i128,
) -> usize {
    (0..k.len())
        .filter(|&i| allowed(i, k[i]))
        .min_by_key(|&i| (key(i, k[i]), i))
        .unwrap_or(0)
}

fn to_i128(v: usize) -> i128 {
    i128::try_from(v).unwrap_or(i128::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lamina_core::AlbumEdge;

    #[test]
    fn proporcional_com_minimo_de_uma() {
        assert_eq!(allocate(&[8; 5], &[8; 10]), Ok(vec![2; 5]));
        assert_eq!(allocate(&[30, 6], &[8; 6]), Ok(vec![5, 1]));
        assert_eq!(allocate(&[10, 1], &[8; 3]), Ok(vec![2, 1]));
        assert_eq!(allocate(&[6, 3], &[8; 2]), Ok(vec![1, 1]));
    }

    #[test]
    fn capacidade_das_pontas_com_pagina_simples() {
        let caps = capacities(
            4,
            Edges::new(AlbumEdge::FullSpread, AlbumEdge::SinglePage),
            4,
        );
        assert_eq!(caps, [8, 8, 8, 4]);
        let one = capacities(
            1,
            Edges::new(AlbumEdge::SinglePage, AlbumEdge::SinglePage),
            4,
        );
        assert_eq!(one, [4]);
    }

    #[test]
    fn momento_que_nao_cabe_toma_lamina_de_quem_tem_folga() {
        // 5 + 3 + 6 fotos em 4 lâminas; a primeira só tem uma página (capacidade 4).
        // A cota dá [1, 1, 2]: o primeiro momento (5 fotos) não cabe em 4 e toma uma lâmina
        // do terceiro, que ainda comporta suas 6 fotos em 8.
        assert_eq!(allocate(&[5, 3, 6], &[4, 8, 8, 8]), Ok(vec![2, 1, 1]));
    }

    #[test]
    fn sem_doador_possivel_aponta_o_momento() {
        // 9 + 2 fotos em 2 lâminas de 8: o primeiro não cabe e ninguém pode ceder.
        assert_eq!(allocate(&[9, 2], &[8, 8]), Err(0));
    }

    #[test]
    fn minimo_de_laminas() {
        assert_eq!(min_spreads(100, Edges::default(), 4), 13);
        let single = Edges::new(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
        assert_eq!(min_spreads(14, single, 4), 3);
    }
}
