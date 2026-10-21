//! As miniaturas da janela (spec TR, regras 6 a 8). Um trabalhador lê cada miniatura do cache
//! fora da linha da janela, e cada lista diz quais fotos mostra agora ([`ThumbPlan::want`]) ou
//! que não mostra mais nenhuma ([`ThumbPlan::release`]). Ficam na memória as que alguma lista
//! pede e, das outras, só as [`THUMBS_KEPT`] usadas por último.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use lamina_core::PhotoId;

/// Quantas miniaturas que nenhuma lista pede ficam na memória (~200 KB cada, ~60 MB no total):
/// voltar a elas é imediato, e o resto sai.
pub const THUMBS_KEPT: usize = 300;

/// Quem pede miniaturas na janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThumbList {
    /// A faixa de lâminas, a linha do tempo, o painel de layouts e a lâmina grande: as fotos
    /// usadas nas lâminas e as selecionadas para o painel de layouts.
    Album,
    /// O painel de fotos: só as linhas da grade à vista e a margem (spec PF, regra 14).
    Browser,
}

/// O que a janela faz depois de um pedido.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThumbOrders {
    /// Pedir ao trabalhador, na ordem da lista.
    pub load: Vec<PhotoId>,
    /// Soltar da memória: passaram do limite das que ninguém pede.
    pub drop: Vec<PhotoId>,
    /// Tirar da fila do trabalhador: pedidas e ainda não lidas, que nenhuma lista pede mais
    /// (rolar rápido não enfileira leituras velhas; spec PF, regra 14).
    pub cancel: Vec<PhotoId>,
}

/// O plano das miniaturas: quem pede o quê, o que está na memória e o que está a caminho.
#[derive(Debug, Default)]
pub struct ThumbPlan {
    wanted: BTreeMap<ThumbList, BTreeSet<PhotoId>>,
    /// Na memória da janela.
    loaded: BTreeSet<PhotoId>,
    /// Pedidas ao trabalhador, ainda não chegaram.
    asked: BTreeSet<PhotoId>,
    /// Sem miniatura no cache: só são pedidas de novo depois de [`ThumbPlan::recheck`].
    missing: BTreeSet<PhotoId>,
    /// As da memória que nenhuma lista pede, da usada há mais tempo para a mais recente.
    idle: VecDeque<PhotoId>,
}

impl ThumbPlan {
    /// A lista `list` mostra agora `photos`: as que ainda não estão na memória nem a caminho são
    /// pedidas (as que a importação acrescentou, por exemplo), e as que passaram do limite saem.
    pub fn want(&mut self, list: ThumbList, photos: &[PhotoId]) -> ThumbOrders {
        let now: BTreeSet<PhotoId> = photos.iter().copied().collect();
        let before = self.wanted.insert(list, now.clone()).unwrap_or_default();
        let mut cancel = Vec::new();
        for id in before.difference(&now) {
            if self.is_wanted(*id) {
                continue;
            }
            if self.loaded.contains(id) {
                self.idle.push_back(*id);
            } else if self.asked.remove(id) {
                cancel.push(*id);
            }
        }
        self.idle.retain(|id| !now.contains(id));
        let mut load = Vec::new();
        for &id in photos {
            let known = self.loaded.contains(&id) || self.missing.contains(&id);
            if !known && self.asked.insert(id) {
                load.push(id);
            }
        }
        ThumbOrders {
            load,
            drop: self.trim(),
            cancel,
        }
    }

    /// A lista `list` não mostra mais nada (fechou, ou a grade rolou para longe).
    pub fn release(&mut self, list: ThumbList) -> Vec<PhotoId> {
        self.want(list, &[]).drop
    }

    /// Chegou do trabalhador a miniatura de `id` (`found`: havia no cache). Devolve se a janela
    /// guarda a imagem e as que saem da memória por causa dela.
    pub fn arrived(&mut self, id: PhotoId, found: bool) -> (bool, Vec<PhotoId>) {
        if !self.asked.remove(&id) {
            return (false, Vec::new());
        }
        if !found {
            self.missing.insert(id);
            return (false, Vec::new());
        }
        self.loaded.insert(id);
        if !self.is_wanted(id) {
            self.idle.push_back(id);
        }
        (true, self.trim())
    }

    /// O cache ganhou imagens: as que faltavam e alguma lista pede são pedidas de novo.
    pub fn recheck(&mut self) -> Vec<PhotoId> {
        let missing = std::mem::take(&mut self.missing);
        let load: Vec<PhotoId> = missing
            .into_iter()
            .filter(|id| self.is_wanted(*id))
            .collect();
        self.asked.extend(&load);
        load
    }

    /// Outro álbum: tudo é esquecido (a janela solta as imagens).
    pub fn forget(&mut self) {
        *self = ThumbPlan::default();
    }

    /// Quantas foram pedidas e ainda não chegaram.
    pub fn pending(&self) -> usize {
        self.asked.len()
    }

    /// Quantas estão na memória da janela.
    pub fn loaded(&self) -> usize {
        self.loaded.len()
    }

    fn is_wanted(&self, id: PhotoId) -> bool {
        self.wanted.values().any(|set| set.contains(&id))
    }

    /// Solta as ociosas mais antigas até sobrarem [`THUMBS_KEPT`].
    fn trim(&mut self) -> Vec<PhotoId> {
        let extra = self.idle.len().saturating_sub(THUMBS_KEPT);
        let out: Vec<PhotoId> = self.idle.drain(..extra).collect();
        for id in &out {
            self.loaded.remove(id);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As fotos de números `range` de um álbum de teste.
    fn ids(range: std::ops::Range<u64>) -> Vec<PhotoId> {
        let cm = lamina_core::Mm::from_cm(30.0);
        let dpi = lamina_core::Dpi::new(300).unwrap();
        let mut project =
            lamina_core::Project::new("teste", lamina_core::AlbumSpec::new(cm, cm, dpi).unwrap());
        (0..range.end)
            .map(|i| project.add_photo(format!("{i}.jpg"), 10, 10))
            .filter(|id| range.contains(&id.get()))
            .collect()
    }

    /// Pede e recebe todas.
    fn load_all(plan: &mut ThumbPlan, list: ThumbList, photos: &[PhotoId]) {
        let orders = plan.want(list, photos);
        arrive_all(plan, &orders.load);
    }

    /// Todas as pedidas chegam do trabalhador.
    fn arrive_all(plan: &mut ThumbPlan, photos: &[PhotoId]) {
        for &id in photos {
            assert!(plan.arrived(id, true).0);
        }
    }

    #[test]
    fn tr_b7_miniaturas_pedidas_por_lista() {
        let mut plan = ThumbPlan::default();
        let all = ids(1..11);
        let orders = plan.want(ThumbList::Album, &all);
        assert_eq!(orders.load, all, "todas, na ordem da lista");
        // O navegador pede as mesmas: já estão a caminho, nada é pedido duas vezes.
        assert!(plan.want(ThumbList::Browser, &all[..4]).load.is_empty());
        assert_eq!(plan.pending(), 10);
        arrive_all(&mut plan, &all);
        assert_eq!((plan.pending(), plan.loaded()), (0, 10));
        // Uma miniatura que não está no cache não é pedida de novo a cada atualização.
        let extra = ids(11..12);
        plan.want(ThumbList::Album, &[all.clone(), extra.clone()].concat());
        assert_eq!(plan.arrived(extra[0], false), (false, Vec::new()));
        assert!(plan.want(ThumbList::Album, &extra).load.is_empty());
        // Até o cache ganhar imagens.
        assert_eq!(plan.recheck(), extra);
        // Chegada de um pedido esquecido (outro álbum) não entra.
        plan.forget();
        assert_eq!(plan.arrived(all[0], true), (false, Vec::new()));
    }

    #[test]
    fn tr_b8_miniaturas_soltas_ficam_no_limite() {
        let mut plan = ThumbPlan::default();
        let many = ids(1..1001);
        load_all(&mut plan, ThumbList::Browser, &many);
        assert_eq!(plan.loaded(), 1000, "as pedidas por uma lista ficam todas");
        // A grade rolou para outras fotos (spec PF): as que ninguém pede passam a ser ociosas.
        let near = ids(1001..1051);
        let orders = plan.want(ThumbList::Browser, &near);
        assert_eq!(orders.load, near);
        assert_eq!(orders.drop.len(), 1000 - THUMBS_KEPT);
        // Sai a usada há mais tempo; as mais recentes ficam.
        assert_eq!(orders.drop[0], many[0]);
        assert_eq!(plan.loaded(), THUMBS_KEPT);
        // Uma pedida por outra lista nunca sai.
        let album = ids(1..3);
        let mut plan = ThumbPlan::default();
        load_all(&mut plan, ThumbList::Album, &album);
        load_all(&mut plan, ThumbList::Browser, &many[2..]);
        let dropped = plan.release(ThumbList::Browser);
        assert!(dropped.iter().all(|id| !album.contains(id)));
        assert_eq!(plan.loaded(), THUMBS_KEPT + album.len());
    }

    #[test]
    fn tr_b9_importar_pede_so_as_miniaturas_novas() {
        let mut plan = ThumbPlan::default();
        let before = ids(1..301);
        load_all(&mut plan, ThumbList::Album, &before);
        // A importação acrescenta 700 fotos: só elas vão ao trabalhador, as 300 continuam.
        let after = ids(1..1001);
        let orders = plan.want(ThumbList::Album, &after);
        assert_eq!(orders.load, ids(301..1001));
        assert!(orders.drop.is_empty());
        assert_eq!(plan.loaded(), 300);
    }
}
