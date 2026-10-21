//! A revisão de cada lâmina do rolo (spec RL, regra 6): sobe quando o que a imagem dela mostra
//! muda. Reordenar não sobe nada, salvo nas pontas com página simples (as páginas ativas mudam).

use std::collections::{BTreeMap, BTreeSet};

use lamina_core::{AlbumSpec, SpreadId};

use super::rows::StripRow;

/// A revisão de cada lâmina: sobe quando o que a imagem dela mostra muda (regra 6). A chave é a
/// linha da faixa sem o aviso, mais a especificação do álbum.
#[derive(Debug, Default)]
pub struct Revisions {
    spec: Option<AlbumSpec>,
    rows: BTreeMap<SpreadId, (StripRow, u64)>,
    order: Vec<SpreadId>,
}

impl Revisions {
    /// Lê as linhas da faixa (na ordem das lâminas) e devolve as lâminas cuja revisão subiu (as
    /// novas também).
    pub fn update(&mut self, rows: &[(SpreadId, StripRow)], spec: &AlbumSpec) -> Vec<SpreadId> {
        let respec = self.spec.as_ref() != Some(spec);
        if respec {
            self.spec = Some(spec.clone());
        }
        self.order = rows.iter().map(|(id, _)| *id).collect();
        let mut changed = Vec::new();
        for (id, row) in rows {
            match self.rows.get_mut(id) {
                Some((old, _)) if !respec && same_image(old, row) => {}
                Some((old, rev)) => {
                    (*old, *rev) = (row.clone(), *rev + 1);
                    changed.push(*id);
                }
                None => {
                    self.rows.insert(*id, (row.clone(), 1));
                    changed.push(*id);
                }
            }
        }
        let alive: BTreeSet<SpreadId> = self.order.iter().copied().collect();
        self.rows.retain(|id, _| alive.contains(id));
        changed
    }

    /// A revisão da lâmina `id` (`None` se ela não existe).
    pub fn of(&self, id: SpreadId) -> Option<u64> {
        self.rows.get(&id).map(|(_, rev)| *rev)
    }

    /// As lâminas, na ordem do álbum.
    pub fn order(&self) -> &[SpreadId] {
        &self.order
    }

    /// Outro álbum: as revisões começam do zero (RL-B22).
    pub fn clear(&mut self) {
        *self = Revisions::default();
    }
}

/// As duas linhas mostram a mesma imagem: tudo igual, menos o aviso (o "!" é da janela).
fn same_image(a: &StripRow, b: &StripRow) -> bool {
    a.active == b.active && a.missing == b.missing && a.adjusts == b.adjusts && a.spread == b.spread
}
