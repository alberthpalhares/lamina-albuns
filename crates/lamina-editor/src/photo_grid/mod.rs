//! O painel de fotos em grade (spec PF): as contas sem janela. A grade mostra as fotos da aba em
//! linhas de N peças quadradas de lado Z (o zoom), com 8 px de vão; aqui ficam as colunas, as
//! linhas visíveis, o laço e a ordem em que as miniaturas são pedidas. O zoom, a altura do painel
//! e a rolagem do laço ficam em [`panel`].

use std::collections::BTreeSet;
use std::ops::Range;

use crate::motion::RowOp;

mod panel;

pub use panel::{
    HEIGHT_SHARE, LASSO_EDGE, LASSO_SPEED, STAGE_MIN, ZOOM_DEFAULT, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP,
    ZOOM_WHEEL_STEP, clamp_zoom, default_height, height_limits, lasso_scroll, rows_height,
    shown_height, wheel_zoom, whole_px, zoom_at_pointer, zoom_keeping_top,
};

/// Vão entre as peças e margem em volta (px lógicos).
pub const GRID_GAP: f32 = 8.0;
/// A linha do nome embaixo da peça.
pub const NAME_LINE: f32 = 18.0;
/// O nome só aparece com o zoom a partir daqui.
pub const NAME_ZOOM: u32 = 96;

/// A parte inteira de uma medida da tela (nunca negativa).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole(x: f32) -> usize {
    x.max(0.0).floor() as usize
}

/// A grade de `count` fotos numa largura e num zoom: colunas, altura das linhas e posições.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub count: usize,
    pub columns: usize,
    pub zoom: u32,
    /// O lado da peça: o zoom, ou menos num painel mais estreito que uma peça.
    pub tile: f32,
}

impl Grid {
    /// Colunas N = máximo de 1 e ⌊(L − 8) / (Z + 8)⌋ (seção 4).
    pub fn new(count: usize, width: f32, zoom: u32) -> Grid {
        let zoom = clamp_zoom(i64::from(zoom));
        let z = zoom as f32;
        Grid {
            count,
            columns: whole((width - GRID_GAP) / (z + GRID_GAP)).max(1),
            zoom,
            tile: z.min(width - 2.0 * GRID_GAP).max(1.0),
        }
    }

    /// Com nome embaixo da peça?
    pub fn named(&self) -> bool {
        self.zoom >= NAME_ZOOM
    }

    /// A altura de uma linha: H = Z + 8 + (18 com o nome).
    pub fn row_height(&self) -> f32 {
        self.tile + GRID_GAP + if self.named() { NAME_LINE } else { 0.0 }
    }

    /// Quantas linhas (a última pode ter menos fotos).
    pub fn rows(&self) -> usize {
        self.count.div_ceil(self.columns)
    }

    /// As fotos da linha `row` (posições na lista mostrada).
    pub fn row_range(&self, row: usize) -> Range<usize> {
        let start = (row * self.columns).min(self.count);
        start..(start + self.columns).min(self.count)
    }

    /// A linha da foto `index`.
    pub fn row_of(&self, index: usize) -> usize {
        index / self.columns
    }

    /// A foto na linha `row` e na coluna `column`, se existe.
    pub fn index_at(&self, row: usize, column: usize) -> Option<usize> {
        let index = row * self.columns + column;
        (column < self.columns && index < self.count).then_some(index)
    }

    /// A altura de todas as linhas, com a margem de baixo.
    pub fn content_height(&self) -> f32 {
        GRID_GAP + self.rows() as f32 * self.row_height()
    }

    /// A rolagem mais funda para um painel de altura `height`.
    pub fn max_scroll(&self, height: f32) -> f32 {
        (self.content_height() - height).max(0.0)
    }

    /// A linha no topo com a rolagem `scroll` (a primeira que aparece).
    pub fn top_row(&self, scroll: f32) -> usize {
        whole(scroll / self.row_height()).min(self.rows().saturating_sub(1))
    }

    /// As linhas que aparecem num painel de altura `height` com a rolagem `scroll`.
    pub fn visible(&self, scroll: f32, height: f32) -> Range<usize> {
        if self.count == 0 {
            return 0..0;
        }
        let first = self.top_row(scroll);
        let bottom = ((scroll.max(0.0) + height.max(0.0)) / self.row_height()).ceil();
        first..whole(bottom).clamp(first + 1, self.rows())
    }

    /// A rolagem que põe a linha da foto `index` no topo, sem passar do fim.
    pub fn scroll_to(&self, index: usize, height: f32) -> f32 {
        let top = self.row_of(index) as f32 * self.row_height();
        top.min(self.max_scroll(height))
    }

    /// A coluna no ponto `x` do painel (a mais perto, dentro da grade).
    pub fn column_at(&self, x: f32) -> usize {
        whole((x - GRID_GAP) / (self.tile + GRID_GAP)).min(self.columns - 1)
    }

    /// As fotos que o laço de `a` a `b` (pontos no conteúdo da grade, com a rolagem) cruza, na
    /// ordem da lista (regra 9). A peça é a miniatura com a linha do nome.
    pub fn lasso(&self, a: (f32, f32), b: (f32, f32)) -> Vec<usize> {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let (step, h) = (self.tile + GRID_GAP, self.row_height());
        let crosses_row = |r: usize| y1 > r as f32 * h + GRID_GAP && y0 < (r + 1) as f32 * h;
        let crosses_column = |c: usize| {
            let left = GRID_GAP + c as f32 * step;
            x1 > left && x0 < left + self.tile
        };
        let rows = (0..self.rows()).filter(|r| crosses_row(*r));
        rows.flat_map(|r| {
            (0..self.columns)
                .filter(|c| crosses_column(*c))
                .filter_map(move |c| self.index_at(r, c))
        })
        .collect()
    }

    /// A ordem das miniaturas (regra 14): as linhas visíveis de cima para baixo, depois a margem
    /// (uma tela acima e uma abaixo, pelo menos duas linhas), da mais perto para a mais longe,
    /// alternando acima e abaixo. Devolve as posições das fotos na lista mostrada.
    pub fn thumb_order(&self, visible: Range<usize>) -> Vec<usize> {
        self.thumb_rows(visible)
            .into_iter()
            .flat_map(|r| self.row_range(r))
            .collect()
    }

    /// As linhas da [`Grid::thumb_order`], na mesma ordem.
    pub fn thumb_rows(&self, visible: Range<usize>) -> Vec<usize> {
        let margin = visible.len().max(2);
        let mut rows: Vec<usize> = visible.clone().collect();
        for k in 1..=margin {
            rows.extend(visible.start.checked_sub(k));
            let below = visible.end + k - 1;
            rows.extend((below < self.rows()).then_some(below));
        }
        rows
    }

    /// As fotos de visível ± margem (as que podem ter miniatura na grade).
    pub fn window(&self, visible: Range<usize>) -> Range<usize> {
        let rows = self.thumb_rows(visible);
        let first = rows.iter().min().map_or(0, |r| self.row_range(*r).start);
        let last = rows.iter().max().map_or(0, |r| self.row_range(*r).end);
        first..last
    }

    /// As linhas da grade que mudam com as operações `ops` da lista de fotos (MV-B16 na grade):
    /// trocar uma foto muda só a linha dela; inserir ou remover refaz da linha até o fim.
    pub fn touched_rows(&self, ops: &[RowOp]) -> Vec<usize> {
        let mut rows = BTreeSet::new();
        for op in ops {
            match *op {
                RowOp::Set { at, .. } => {
                    rows.insert(self.row_of(at));
                }
                RowOp::Insert { at, .. } | RowOp::Remove { at } => {
                    rows.extend(self.row_of(at)..self.rows().max(self.row_of(at) + 1));
                }
            }
        }
        rows.into_iter().collect()
    }
}

/// As linhas da grade: pedaços de N fotos (regra 1).
pub fn grid_rows(count: usize, width: f32, zoom: u32) -> Vec<Range<usize>> {
    let grid = Grid::new(count, width, zoom);
    (0..grid.rows()).map(|r| grid.row_range(r)).collect()
}
