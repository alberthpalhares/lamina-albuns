//! Sugestão do número de lâminas quando o usuário não informa (spec CB, regra 2): cerca de 4
//! fotos por lâmina, como na referência (estudo E2: 40 → 10; 8 → 2).

use lamina_core::Edges;

use super::PlanOptions;
use super::allocate::min_spreads;

/// Fotos por lâmina na sugestão.
pub const PHOTOS_PER_SPREAD: usize = 4;

/// `arredondar(fotos ÷ 4)`, no mínimo 1, nunca menos que as lâminas em que as fotos cabem nem
/// mais que uma lâmina por foto.
pub fn suggest_spreads(photos: usize, edges: Edges, options: &PlanOptions) -> usize {
    if photos == 0 {
        return 1;
    }
    let rounded = (photos + PHOTOS_PER_SPREAD / 2) / PHOTOS_PER_SPREAD;
    let fits = min_spreads(photos, edges, options.max_per_page.max(1));
    rounded.max(fits).clamp(1, photos)
}

#[cfg(test)]
mod tests {
    use lamina_core::AlbumEdge;

    use super::*;

    fn suggest(photos: usize) -> usize {
        suggest_spreads(photos, Edges::default(), &PlanOptions::default())
    }

    #[test]
    fn cerca_de_quatro_fotos_por_lamina() {
        assert_eq!(suggest(40), 10);
        assert_eq!(suggest(8), 2);
        assert_eq!(suggest(9), 2);
        assert_eq!(suggest(10), 3);
        assert_eq!(suggest(1), 1);
        assert_eq!(suggest(0), 1);
    }

    #[test]
    fn nunca_menos_que_as_laminas_em_que_as_fotos_cabem() {
        // Com 1 foto por página, 10 fotos precisam de 5 lâminas (a sugestão pura daria 3).
        let options = PlanOptions {
            max_per_page: 1,
            ..PlanOptions::default()
        };
        assert_eq!(suggest_spreads(10, Edges::default(), &options), 5);
        let single = Edges::new(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
        assert_eq!(suggest_spreads(10, single, &options), 6);
    }
}
