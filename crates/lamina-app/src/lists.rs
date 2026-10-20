//! Listas da janela que mudam linha a linha (spec MV, regra 3): a faixa de lâminas, a linha do
//! tempo e o navegador ficam com o mesmo modelo, e cada atualização só troca, insere ou remove as
//! linhas que o editor diz (`row_ops`). Assim a tela não recria tudo a cada edição.

use std::rc::Rc;

use lamina_editor::motion::{RowOp, row_ops};
use slint::{Model, ModelRc, VecModel};

/// Um modelo da janela e as linhas (chave, valor) que ele mostra agora.
pub struct Synced<K, V, T> {
    rows: Vec<(K, V)>,
    model: Rc<VecModel<T>>,
    /// Refazer tudo na próxima vez (álbum novo, miniaturas novas).
    stale: bool,
}

impl<K: PartialEq, V: PartialEq, T: Clone + 'static> Synced<K, V, T> {
    pub fn new() -> Self {
        Synced {
            rows: Vec::new(),
            model: Rc::new(VecModel::default()),
            stale: true,
        }
    }

    /// O modelo para ligar na janela (uma vez só: ele é sempre o mesmo).
    pub fn model(&self) -> ModelRc<T> {
        ModelRc::from(self.model.clone())
    }

    /// A próxima atualização refaz todas as linhas.
    pub fn reset(&mut self) {
        self.stale = true;
    }

    /// Leva o modelo às linhas `rows`; `make(i)` faz a linha `i` da lista nova. Devolve quantas
    /// linhas mudaram.
    pub fn sync(&mut self, rows: Vec<(K, V)>, mut make: impl FnMut(usize) -> T) -> usize {
        if std::mem::take(&mut self.stale) {
            self.model
                .set_vec((0..rows.len()).map(&mut make).collect::<Vec<T>>());
            let n = rows.len();
            self.rows = rows;
            return n;
        }
        let ops = row_ops(&self.rows, &rows);
        for op in &ops {
            match *op {
                RowOp::Set { at, from } => self.model.set_row_data(at, make(from)),
                RowOp::Insert { at, from } => self.model.insert(at, make(from)),
                RowOp::Remove { at } => {
                    self.model.remove(at);
                }
            }
        }
        self.rows = rows;
        ops.len()
    }
}

/// As listas da janela que mudam linha a linha e a chave das prévias do painel de layouts.
pub struct Lists {
    pub strip: Synced<lamina_core::SpreadId, lamina_editor::motion::StripRow, crate::SpreadData>,
    pub line: Synced<lamina_editor::motion::TimelineKey, usize, crate::TimelineItem>,
    pub browser: Synced<lamina_core::PhotoId, lamina_editor::motion::PhotoRow, crate::PhotoData>,
    /// As prévias do painel mostradas agora (MV-B17): igual, o painel não é refeito.
    pub panel: Option<lamina_editor::motion::PanelKey>,
}

impl Lists {
    /// As listas, já ligadas na janela.
    pub fn new(window: &crate::AppWindow) -> Self {
        let lists = Lists {
            strip: Synced::new(),
            line: Synced::new(),
            browser: Synced::new(),
            panel: None,
        };
        window.set_spreads(lists.strip.model());
        window.set_timeline(lists.line.model());
        window.set_photos(lists.browser.model());
        lists
    }

    /// Álbum novo ou miniaturas novas: tudo é refeito na próxima atualização.
    pub fn reset(&mut self) {
        self.strip.reset();
        self.line.reset();
        self.browser.reset();
        self.panel = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_as_linhas_que_mudaram_sao_feitas_de_novo() {
        let mut list: Synced<char, u8, String> = Synced::new();
        let rows = |s: &str| s.chars().map(|c| (c, 0_u8)).collect::<Vec<_>>();
        assert_eq!(list.sync(rows("abc"), |i| format!("{i}")), 3);
        let mut made = Vec::new();
        list.sync(rows("abxc"), |i| {
            made.push(i);
            format!("novo {i}")
        });
        assert_eq!(made, vec![2]);
        assert_eq!(list.model.row_count(), 4);
        assert_eq!(list.model.row_data(2).as_deref(), Some("novo 2"));
        assert_eq!(list.model.row_data(3).as_deref(), Some("2"));
        list.reset();
        assert_eq!(list.sync(rows("ab"), |i| format!("{i}")), 2);
        assert_eq!(list.model.row_count(), 2);
    }
}
