//! As linhas do rolo na janela e a porta dos envios (spec RL, regra 9; resultado do G0): só as
//! lâminas à vista e uma de cada lado ficam no modelo (a linha sai quando passa de duas), e no
//! máximo uma coluna de imagem nova entra por quadro desenhado. A coluna à vista sem imagem vem
//! primeiro; sem nenhuma, uma de fora da vista sobe antes de aparecer (e é devolvida se surgir uma
//! à vista). O envio só conta como feito quando sai num quadro com a coluna à vista.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::rc::Rc;

use lamina_core::SpreadId;
use slint::{Image, Model, VecModel};

use crate::RollCell;
use crate::images::Pixels;

/// Até quantas colunas uma imagem tem (2.048 px em colunas de 350).
pub const MAX_COLUMNS: usize = 6;

/// Uma imagem guardada: a chave (revisão, nível e largura) e as colunas.
#[derive(Debug, Clone)]
pub struct Stored {
    pub key: (u64, u8, u32),
    pub columns: Vec<Pixels>,
}

/// O envio de uma coluna que ainda não saiu num quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pending {
    spread: SpreadId,
    column: usize,
    offscreen: bool,
}

/// O que uma linha do modelo mostra, do lado do Rust.
#[derive(Debug, Clone)]
struct Row {
    index: usize,
    spread: SpreadId,
    columns: [Option<(u64, u8, u32)>; MAX_COLUMNS],
}

/// O modelo do rolo e o que cada coluna já recebeu.
pub struct Cells {
    model: Rc<VecModel<RollCell>>,
    rows: Vec<Row>,
    pending: Option<Pending>,
    /// As lâminas que seguram textura: as visíveis ±1 (regra 9).
    want: Option<RangeInclusive<usize>>,
    version: i32,
    /// Quantas colunas subiram (para o `LAMINA_TEMPOS`).
    pub uploads: usize,
}

impl Cells {
    pub fn new() -> Self {
        Cells {
            model: Rc::new(VecModel::default()),
            rows: Vec::new(),
            pending: None,
            want: None,
            version: 0,
            uploads: 0,
        }
    }

    pub fn model(&self) -> slint::ModelRc<RollCell> {
        slint::ModelRc::from(self.model.clone())
    }

    /// Esquece tudo (álbum novo).
    pub fn clear(&mut self) {
        self.model.set_vec(Vec::new());
        self.rows.clear();
        self.pending = None;
    }

    /// Leva o modelo às lâminas `keep` (as que ficam) e `want` (as que entram), com `order` dando
    /// a lâmina de cada posição e `warned` o aviso de cada uma.
    pub fn sync(
        &mut self,
        (keep, want): (RangeInclusive<usize>, RangeInclusive<usize>),
        order: &[SpreadId],
        warned: &[bool],
    ) {
        let mut i = 0;
        while i < self.rows.len() {
            let row = &self.rows[i];
            let moved = order.get(row.index) != Some(&row.spread);
            if moved || !keep.contains(&row.index) {
                if self.pending.is_some_and(|p| p.spread == row.spread) {
                    self.pending = None;
                }
                self.rows.remove(i);
                self.model.remove(i);
            } else {
                i += 1;
            }
        }
        self.want = Some(want.clone());
        // Linhas que ficaram além de ±1 soltam a imagem (a textura sai da placa).
        let far: Vec<SpreadId> = self
            .rows
            .iter()
            .filter(|r| !want.contains(&r.index) && r.columns.iter().any(Option::is_some))
            .map(|r| r.spread)
            .collect();
        for spread in far {
            self.forget(spread);
        }
        for index in want {
            let (Some(spread), false) =
                (order.get(index), self.rows.iter().any(|r| r.index == index))
            else {
                continue;
            };
            self.rows.push(Row {
                index,
                spread: *spread,
                columns: [None; MAX_COLUMNS],
            });
            self.model.push(blank(index, warned.get(index).copied()));
        }
        for (r, row) in self.rows.iter().enumerate() {
            let warning = warned.get(row.index).copied().unwrap_or(false);
            if let Some(mut cell) = self.model.row_data(r)
                && cell.warning != warning
            {
                cell.warning = warning;
                self.model.set_row_data(r, cell);
            }
        }
    }

    /// Tira as colunas da lâmina `spread` (a imagem saiu da memória).
    pub fn forget(&mut self, spread: SpreadId) {
        if let Some(r) = self.rows.iter().position(|row| row.spread == spread) {
            self.rows[r].columns = [None; MAX_COLUMNS];
            let warning = self.model.row_data(r).map(|c| c.warning);
            self.model
                .set_row_data(r, blank(self.rows[r].index, warning));
            if self.pending.is_some_and(|p| p.spread == spread) {
                self.pending = None;
            }
        }
    }

    /// Depois de um quadro desenhado: o envio pendente saiu? E, se a porta está livre, a próxima
    /// coluna. `seen(índice, coluna)` diz se a coluna está à vista; `near(índice)`, a distância
    /// ao centro. Devolve se uma coluna nova entrou.
    pub fn frame(
        &mut self,
        store: &BTreeMap<SpreadId, Stored>,
        seen: &dyn Fn(usize, usize, usize) -> bool,
        near: &dyn Fn(usize) -> f64,
    ) -> bool {
        if let Some(p) = self.pending
            && self.row_of(p.spread).is_some_and(|r| {
                let row = &self.rows[r];
                seen(row.index, p.column, columns_of(store, row.spread))
            })
        {
            self.pending = None;
        }
        let visible = self.next(store, seen, near, true);
        if let Some(p) = self.pending {
            if !(p.offscreen && visible.is_some()) {
                return false;
            }
            // A coluna de fora da vista ainda não subiu: devolve e atende a que está à vista.
            self.unset(p.spread, p.column);
            self.pending = None;
        }
        let next = visible.or_else(|| self.next(store, seen, near, false));
        let Some((spread, column, offscreen)) = next else {
            return false;
        };
        self.assign(store, spread, column);
        self.pending = Some(Pending {
            spread,
            column,
            offscreen,
        });
        true
    }

    fn row_of(&self, spread: SpreadId) -> Option<usize> {
        self.rows.iter().position(|r| r.spread == spread)
    }

    /// A coluna sem a imagem guardada mais perto do centro, à vista ou fora dela.
    fn next(
        &self,
        store: &BTreeMap<SpreadId, Stored>,
        seen: &dyn Fn(usize, usize, usize) -> bool,
        near: &dyn Fn(usize) -> f64,
        visible: bool,
    ) -> Option<(SpreadId, usize, bool)> {
        let mut best: Option<(f64, SpreadId, usize)> = None;
        let want = self.want.clone();
        for row in self
            .rows
            .iter()
            .filter(|r| want.as_ref().is_some_and(|w| w.contains(&r.index)))
        {
            let Some(s) = store.get(&row.spread) else {
                continue;
            };
            for (k, have) in row.columns.iter().enumerate().take(s.columns.len()) {
                let fits = *have != Some(s.key) && seen(row.index, k, s.columns.len()) == visible;
                let d = near(row.index);
                if fits && best.is_none_or(|(bd, ..)| d < bd) {
                    best = Some((d, row.spread, k));
                }
            }
        }
        best.map(|(_, spread, k)| (spread, k, !visible))
    }

    fn assign(&mut self, store: &BTreeMap<SpreadId, Stored>, spread: SpreadId, column: usize) {
        let (Some(r), Some(s)) = (self.row_of(spread), store.get(&spread)) else {
            return;
        };
        let Some(pixels) = s.columns.get(column) else {
            return;
        };
        let Some(mut cell) = self.model.row_data(r) else {
            return;
        };
        self.version += 1;
        let width: u32 = s.columns.iter().map(Pixels::width).sum();
        cell.iw = i32::try_from(width).unwrap_or(1);
        set_column(
            &mut cell,
            column,
            Image::from_rgba8(pixels.clone()),
            self.version,
        );
        self.model.set_row_data(r, cell);
        self.rows[r].columns[column] = Some(s.key);
        self.uploads += 1;
    }

    fn unset(&mut self, spread: SpreadId, column: usize) {
        let Some(r) = self.row_of(spread) else {
            return;
        };
        if let Some(mut cell) = self.model.row_data(r) {
            set_column(&mut cell, column, Image::default(), 0);
            self.model.set_row_data(r, cell);
        }
        self.rows[r].columns[column] = None;
    }
}

/// Quantas colunas a imagem guardada da lâmina tem.
fn columns_of(store: &BTreeMap<SpreadId, Stored>, spread: SpreadId) -> usize {
    store.get(&spread).map_or(0, |s| s.columns.len())
}

fn blank(index: usize, warning: Option<bool>) -> RollCell {
    RollCell {
        index: i32::try_from(index).unwrap_or(0),
        iw: 1,
        warning: warning.unwrap_or(false),
        ..RollCell::default()
    }
}

fn set_column(cell: &mut RollCell, k: usize, img: Image, version: i32) {
    match k {
        0 => (cell.c0, cell.v0) = (img, version),
        1 => (cell.c1, cell.v1) = (img, version),
        2 => (cell.c2, cell.v2) = (img, version),
        3 => (cell.c3, cell.v3) = (img, version),
        4 => (cell.c4, cell.v4) = (img, version),
        _ => (cell.c5, cell.v5) = (img, version),
    }
}

#[cfg(test)]
mod tests {
    use lamina_core::{AlbumSpec, Dpi, Mm, Project};
    use slint::{Rgba8Pixel, SharedPixelBuffer};

    use super::*;

    fn ids(count: usize) -> Vec<SpreadId> {
        let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap());
        let mut project = Project::new("teste", spec.unwrap());
        (0..count).map(|_| project.add_spread()).collect()
    }

    #[test]
    fn rl_b1_linhas_do_rolo_so_com_a_imagem_e_uma_coluna_por_quadro() {
        let order = ids(75);
        let mut cells = Cells::new();
        cells.sync((2..=8, 3..=7), &order, &[false; 75]);
        let model = cells.model();
        let rows: Vec<i32> = (0..model.row_count())
            .filter_map(|r| model.row_data(r))
            .map(|c| c.index)
            .collect();
        assert_eq!(rows, vec![3, 4, 5, 6, 7], "as visíveis e uma de cada lado");
        // A lâmina 6 (índice 5) tem uma imagem de 3 colunas; a porta põe uma coluna por quadro.
        let column = SharedPixelBuffer::<Rgba8Pixel>::new(350, 10);
        let store = BTreeMap::from([(
            order[5],
            Stored {
                key: (1, 1, 1050),
                columns: vec![column.clone(), column.clone(), column],
            },
        )]);
        let seen = |_: usize, _: usize, _: usize| true;
        let near = |i: usize| (i as f64 - 5.0).abs();
        let mut uploads = 0;
        while cells.frame(&store, &seen, &near) {
            uploads += 1;
            assert_eq!(cells.uploads, uploads, "uma coluna por quadro");
        }
        assert_eq!(uploads, 3);
        let cell = (0..model.row_count())
            .filter_map(|r| model.row_data(r))
            .find(|c| c.index == 5)
            .unwrap();
        assert_eq!(
            (cell.iw, cell.v0 > 0, cell.v2 > 0, cell.v3),
            (1050, true, true, 0)
        );
        // Rolou para longe: as linhas saem, e a imagem continua no Rust (no `store`).
        cells.sync((28..=32, 29..=31), &order, &[false; 75]);
        assert_eq!(model.row_count(), 3);
    }
}
