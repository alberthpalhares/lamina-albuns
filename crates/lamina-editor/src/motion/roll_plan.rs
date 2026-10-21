//! O plano do rolo (spec RL, regras 7 a 9; ADR 0012, decisões 4 e 5), no molde do `ScreenPlan`:
//! o que pedir ao trabalhador e em que ordem, o que fazer com cada imagem que chega e quais saem
//! quando a memória passa do teto.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;

use lamina_core::SpreadId;

use super::roll::{
    Landing, ROLL_BYTES, RollAsk, RollGot, RollView, SCREEN_MAX, SMALL_WIDTH, Tier, close,
};
use super::roll_revisions::Revisions;
use crate::screens::Arrival;

/// O plano do rolo (regras 7 a 9), no molde do `ScreenPlan`.
#[derive(Debug)]
pub struct RollPlan {
    epoch: u64,
    cap: usize,
    center: usize,
    /// As lâminas que ainda podem receber imagem (as visíveis ±2); nenhuma antes do primeiro
    /// plano.
    reach: Option<RangeInclusive<usize>>,
    order: Vec<SpreadId>,
    revs: BTreeMap<SpreadId, u64>,
    kept: BTreeMap<SpreadId, RollGot>,
    /// Imagens incompletas que podem ser pedidas de novo (o cache mudou).
    retry: BTreeSet<SpreadId>,
    /// Chegaram durante um deslize: entram quando ele termina (regra 8).
    held: BTreeSet<SpreadId>,
}

impl Default for RollPlan {
    fn default() -> Self {
        RollPlan::with_cap(ROLL_BYTES)
    }
}

impl RollPlan {
    /// Um plano com o teto de memória `cap` bytes.
    pub fn with_cap(cap: usize) -> Self {
        RollPlan {
            epoch: 0,
            cap,
            center: 0,
            reach: None,
            order: Vec::new(),
            revs: BTreeMap::new(),
            kept: BTreeMap::new(),
            retry: BTreeSet::new(),
            held: BTreeSet::new(),
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Outro álbum: o que estava guardado e os pedidos em curso deixam de valer (RL-B22).
    pub fn new_album(&mut self) {
        let epoch = self.epoch + 1;
        *self = RollPlan::with_cap(self.cap);
        self.epoch = epoch;
    }

    /// Os pedidos que faltam para o que o rolo mostra, na ordem da regra 7. A fila do trabalhador
    /// troca inteira por esta lista.
    pub fn go(&mut self, view: &RollView, revisions: &Revisions) -> Vec<RollAsk> {
        self.order = revisions.order().to_vec();
        self.revs = self
            .order
            .iter()
            .filter_map(|id| Some((*id, revisions.of(*id)?)))
            .collect();
        let Some(last) = self.order.len().checked_sub(1) else {
            self.reach = None;
            return Vec::new();
        };
        let (first, end) = (*view.visible.start(), (*view.visible.end()).min(last));
        self.center = view.center.min(last);
        self.reach = Some(first.saturating_sub(2)..=(end + 2).min(last));
        let mut around: Vec<usize> = (first.saturating_sub(1)..=(end + 1).min(last))
            .filter(|i| *i != view.current)
            .collect();
        let center = self.center;
        around.sort_by_key(|i| (!(first..=end).contains(i), i.abs_diff(center), *i));
        let screen = view.width.clamp(1, SCREEN_MAX);
        let mut asks: Vec<RollAsk> = around
            .iter()
            .filter_map(|i| self.need(*i, Tier::Small, SMALL_WIDTH))
            .collect();
        asks.extend(
            around
                .iter()
                .filter_map(|i| self.need(*i, Tier::Screen, screen)),
        );
        if view.current_idle {
            asks.extend(self.need(view.current, Tier::Screen, screen));
        }
        asks
    }

    /// O pedido da lâmina `index` no nível `tier`, se a imagem guardada não serve.
    fn need(&self, index: usize, tier: Tier, width: u32) -> Option<RollAsk> {
        let spread = *self.order.get(index)?;
        let revision = *self.revs.get(&spread)?;
        let retry = self.retry.contains(&spread);
        let ok = self.kept.get(&spread).is_some_and(|k| {
            let current = k.ask.revision == revision && !(retry && !k.complete);
            match tier {
                Tier::Small => current,
                Tier::Screen => current && k.ask.tier == Tier::Screen && close(k.ask.width, width),
            }
        });
        (!ok).then_some(RollAsk {
            epoch: self.epoch,
            spread,
            index,
            tier,
            revision,
            width,
        })
    }

    /// Uma imagem chegou; `sliding` diz se o rolo está deslizando (regra 8).
    pub fn arrived(&mut self, got: RollGot, sliding: bool) -> Landing {
        let discard = Landing {
            arrival: Arrival::Discard,
            evict: Vec::new(),
        };
        let id = got.ask.spread;
        let index = self.order.iter().position(|s| *s == id);
        let in_reach = index.is_some_and(|i| self.reach.as_ref().is_some_and(|r| r.contains(&i)));
        let fresh = self.revs.get(&id).is_some_and(|r| got.ask.revision >= *r);
        if got.ask.epoch != self.epoch || !in_reach || !fresh || self.worse(&got) {
            return discard;
        }
        self.kept.insert(id, got);
        self.retry.remove(&id);
        let evict = self.trim();
        for gone in &evict {
            self.held.remove(gone);
        }
        let arrival = if sliding {
            self.held.insert(id);
            Arrival::Hold
        } else {
            self.held.remove(&id);
            Arrival::Show
        };
        Landing { arrival, evict }
    }

    /// O deslize terminou: as imagens seguradas entram agora.
    pub fn slide_ended(&mut self) -> Vec<SpreadId> {
        std::mem::take(&mut self.held).into_iter().collect()
    }

    /// Já há imagem melhor ou igual da mesma revisão (nível maior, ou o mesmo pedido).
    fn worse(&self, got: &RollGot) -> bool {
        self.kept.get(&got.ask.spread).is_some_and(|k| {
            k.ask.revision == got.ask.revision
                && (k.ask.tier > got.ask.tier || (k.ask == got.ask && k.complete))
        })
    }

    /// Passou do teto: sai a mais longe do centro e, no empate, a de maior índice (regra 9).
    fn trim(&mut self) -> Vec<SpreadId> {
        let mut out = Vec::new();
        while self.kept.values().map(|k| k.bytes).sum::<usize>() > self.cap {
            let far = self.kept.keys().copied().max_by_key(|id| {
                let index = self.order.iter().position(|s| s == id);
                index.map_or((usize::MAX, usize::MAX), |i| (i.abs_diff(self.center), i))
            });
            let Some(far) = far else { break };
            self.kept.remove(&far);
            out.push(far);
        }
        out
    }

    /// O cache ganhou imagens: as incompletas podem ser pedidas de novo (regra 4, RL-B9).
    pub fn recheck(&mut self) {
        let incomplete = self.kept.iter().filter(|(_, k)| !k.complete);
        self.retry = incomplete.map(|(id, _)| *id).collect();
    }

    /// A imagem guardada da lâmina `id`, se houver.
    pub fn kept(&self, id: SpreadId) -> Option<&RollGot> {
        self.kept.get(&id)
    }

    /// A lâmina `id` tem a imagem de tela da revisão de agora (regra 13).
    pub fn sharp(&self, id: SpreadId) -> bool {
        let rev = self.revs.get(&id);
        self.kept
            .get(&id)
            .is_some_and(|k| k.ask.tier == Tier::Screen && Some(&k.ask.revision) == rev)
    }

    /// Bytes das imagens guardadas.
    pub fn bytes(&self) -> usize {
        self.kept.values().map(|k| k.bytes).sum()
    }
}
