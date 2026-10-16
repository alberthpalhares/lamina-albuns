//! Como o miolo do álbum começa e termina: com uma lâmina inteira ou com uma página simples.
//!
//! Em muitos álbuns a primeira página do miolo fica sozinha à direita (a esquerda é a guarda
//! da capa) e a última fica sozinha à esquerda. O laboratório informa isso na especificação.

use serde::{Deserialize, Serialize};

/// Como uma ponta do miolo é diagramada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlbumEdge {
    /// A primeira (ou última) lâmina usa as duas páginas.
    #[default]
    FullSpread,
    /// A primeira lâmina só tem a página direita; a última, só a esquerda.
    SinglePage,
}

/// Como o miolo começa e termina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Edges {
    pub start: AlbumEdge,
    pub end: AlbumEdge,
}

/// Quais páginas de uma lâmina recebem conteúdo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePages {
    pub left: bool,
    pub right: bool,
}

impl Edges {
    pub fn new(start: AlbumEdge, end: AlbumEdge) -> Self {
        Edges { start, end }
    }

    /// Número de páginas impressas de um álbum com `spreads` lâminas.
    pub fn page_count(&self, spreads: usize) -> usize {
        let single = |edge| usize::from(edge == AlbumEdge::SinglePage);
        if spreads == 0 {
            return 0;
        }
        (spreads * 2).saturating_sub(single(self.start) + single(self.end))
    }

    /// Páginas ativas da lâmina `index` (começando em 0) num álbum de `spreads` lâminas.
    pub fn active_pages(&self, index: usize, spreads: usize) -> ActivePages {
        let first = index == 0;
        let last = index + 1 == spreads;
        ActivePages {
            left: !(first && self.start == AlbumEdge::SinglePage),
            right: !(last && self.end == AlbumEdge::SinglePage),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE: AlbumEdge = AlbumEdge::SinglePage;
    const FULL: AlbumEdge = AlbumEdge::FullSpread;

    #[test]
    fn lamina_inteira_nas_duas_pontas_usa_todas_as_paginas() {
        let edges = Edges::default();
        assert_eq!(edges.page_count(10), 20);
        assert_eq!(
            edges.active_pages(0, 10),
            ActivePages {
                left: true,
                right: true
            }
        );
    }

    #[test]
    fn pagina_simples_no_inicio_e_no_fim() {
        let edges = Edges::new(SINGLE, SINGLE);
        assert_eq!(edges.page_count(10), 18);
        assert_eq!(
            edges.active_pages(0, 10),
            ActivePages {
                left: false,
                right: true
            }
        );
        assert_eq!(
            edges.active_pages(9, 10),
            ActivePages {
                left: true,
                right: false
            }
        );
        assert_eq!(
            edges.active_pages(4, 10),
            ActivePages {
                left: true,
                right: true
            }
        );
    }

    #[test]
    fn so_o_inicio_em_pagina_simples() {
        let edges = Edges::new(SINGLE, FULL);
        assert_eq!(edges.page_count(10), 19);
        assert!(edges.active_pages(9, 10).right);
    }

    #[test]
    fn album_vazio_ou_de_uma_lamina() {
        let edges = Edges::new(SINGLE, SINGLE);
        assert_eq!(edges.page_count(0), 0);
        assert_eq!(edges.page_count(1), 0);
    }
}
