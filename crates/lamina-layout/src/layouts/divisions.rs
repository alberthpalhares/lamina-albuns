//! Formas de dividir as fotos de um bloco em linhas (ou colunas): todas até 6 fotos (TP) e só
//! as equilibradas acima disso (spec VL, regra 3).

use std::sync::OnceLock;

use lamina_core::Rect;

use super::Arrangement;

/// Até este número de fotos num bloco, todas as divisões (2ⁿ⁻¹); acima, só as equilibradas.
pub const ALL_DIVISIONS_UP_TO: usize = 6;
/// Máximo de linhas (ou colunas) num bloco com divisões equilibradas.
pub const MAX_GROUPS: usize = 8;

/// Maior bloco possível: a lâmina inteira com 12 fotos em cada página.
pub const MAX_BLOCK: usize = 24;

/// Formas de cortar `n` fotos em grupos consecutivos, da com menos grupos para a com mais; com
/// o mesmo número de grupos, em ordem crescente dos tamanhos. Calculadas uma vez só.
pub fn compositions(n: usize) -> &'static [Vec<usize>] {
    static TABLE: OnceLock<Vec<Vec<Vec<usize>>>> = OnceLock::new();
    let table = TABLE.get_or_init(|| (0..=MAX_BLOCK).map(compute).collect());
    table.get(n).map_or(&[], Vec::as_slice)
}

fn compute(n: usize) -> Vec<Vec<usize>> {
    let mut all = if n <= ALL_DIVISIONS_UP_TO {
        every_division(n)
    } else {
        balanced_divisions(n)
    };
    all.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    all
}

fn every_division(n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        return Vec::new();
    }
    (0..1usize << n.saturating_sub(1))
        .map(|cuts| {
            let mut groups = vec![1];
            for i in 0..n.saturating_sub(1) {
                if cuts & (1 << i) != 0 {
                    groups.push(1);
                } else if let Some(last) = groups.last_mut() {
                    *last += 1;
                }
            }
            groups
        })
        .collect()
}

/// Divisões em que o tamanho dos grupos difere no máximo 1 (ex.: 10 → 4+3+3, 3+4+3, 5+5…).
fn balanced_divisions(n: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for k in 1..=n.min(MAX_GROUPS) {
        let (size, extra) = (n / k, n % k);
        for mask in 0..1usize << k {
            if mask.count_ones() as usize == extra {
                out.push((0..k).map(|i| size + (mask >> i & 1)).collect());
            }
        }
    }
    out
}

/// Com mais de 6 fotos, só as divisões com número de linhas (ou colunas) perto do ideal para a
/// proporção do bloco: √(n × altura × proporção média ÷ largura), com folga de 1,5. Longe disso
/// as fotos ficam minúsculas ou o bloco sobra muito; poupa tempo sem perder as melhores.
pub fn near_ideal(area: Rect, aspects: &[f64], arrangement: Arrangement) -> impl Fn(usize) -> bool {
    let n = aspects.len();
    let all = n <= ALL_DIVISIONS_UP_TO;
    let count = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let mean = aspects.iter().sum::<f64>() / count;
    let (w, h) = (area.width.get(), area.height.get());
    let ideal = match arrangement {
        Arrangement::Rows => (count * h * mean / w).sqrt(),
        Arrangement::Columns => (count * w / (h * mean)).sqrt(),
    };
    move |groups: usize| {
        all || (f64::from(u32::try_from(groups).unwrap_or(u32::MAX)) - ideal).abs() <= 1.5
    }
}
