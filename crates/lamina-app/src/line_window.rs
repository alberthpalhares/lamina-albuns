//! A linha do tempo virtualizada (spec RL, regra 22; spec PF, seção 10: "virtualizar a linha do
//! tempo → RL"): a lista inteira continua no `Synced` dela (`lists.line`), com a posição de cada
//! item, e a tela recebe só os itens perto do trecho visível. Com 600 fotos, a linha inteira na
//! tela custava um quadro de cada três na navegação (medido no G1).
//!
//! Cada item fica sempre na mesma vaga do modelo (a posição dele, módulo o número de vagas), como
//! na grade da PF: rolar troca só as vagas dos itens que entraram ou saíram do trecho. Trocar o
//! modelo inteiro refazia as ~50 peças da tela a cada passo da rolagem, subia de novo todas as
//! miniaturas visíveis e reconstruía a árvore de acessibilidade (spec RL, regra 31).

use std::rc::Rc;

use slint::{Model, ModelRc, VecModel};

use crate::TimelineItem;

/// Quanto além do trecho visível, de cada lado, os itens já ficam na tela.
const MARGIN: f32 = 600.0;
/// As medidas de hoje (as mesmas do `TIMELINE` do editor): margem, foto, corte e espaço.
const PADDING: f32 = 6.0;
const PHOTO: f32 = 46.0;
const CUT: f32 = 14.0;
const SPACING: f32 = 2.0;
/// Vagas a mais quando o trecho passa do número de vagas (janela mais larga).
const SLACK: usize = 16;

/// O que a tela mostra da linha do tempo.
pub struct LineWindow {
    model: Rc<VecModel<TimelineItem>>,
    start: f32,
    width: f32,
}

impl LineWindow {
    pub fn new() -> Self {
        LineWindow {
            model: Rc::new(VecModel::default()),
            start: 0.0,
            width: 0.0,
        }
    }

    pub fn model(&self) -> ModelRc<TimelineItem> {
        ModelRc::from(self.model.clone())
    }

    /// O trecho visível mudou (a rolagem ou a largura).
    pub fn set_view(&mut self, start: f32, width: f32) {
        (self.start, self.width) = (start, width);
    }

    /// Leva a tela aos itens de `all` perto do trecho visível; devolve a largura da linha inteira.
    pub fn refresh(&mut self, all: &dyn Model<Data = TimelineItem>) -> f32 {
        let n = all.row_count();
        let end = |i: usize| all.row_data(i).map_or(0.0, |t| t.x + width_of(&t));
        let total = n
            .checked_sub(1)
            .map_or(PADDING * 2.0, |last| end(last) + PADDING);
        let (lo, hi) = (self.start - MARGIN, self.start + self.width + MARGIN);
        let first = partition(n, |i| end(i) < lo);
        let last = partition(n, |i| all.row_data(i).is_some_and(|t| t.x <= hi));
        let range = first..last.max(first);
        if range.len() > self.model.row_count() {
            self.model.set_vec(vec![empty_slot(); range.len() + SLACK]);
        }
        let slots = self.model.row_count();
        for slot in 0..slots {
            let new = slot_item(&range, slot, slots)
                .and_then(|i| all.row_data(i))
                .unwrap_or_else(empty_slot);
            if !self
                .model
                .row_data(slot)
                .is_some_and(|old| same(&old, &new))
            {
                self.model.set_row_data(slot, new);
            }
        }
        total
    }
}

/// O item do trecho que fica na vaga `slot`: o de posição com resto `slot` (cabe um só, porque o
/// trecho nunca passa do número de vagas).
fn slot_item(range: &std::ops::Range<usize>, slot: usize, slots: usize) -> Option<usize> {
    let i = range.start + (slot + slots - range.start % slots) % slots;
    (i < range.end).then_some(i)
}

/// O mesmo item na tela. Duas imagens vazias contam como iguais (no Slint 1.18.1 elas nunca são
/// `==`, e a vaga seria regravada a cada passo da rolagem).
fn same(a: &TimelineItem, b: &TimelineItem) -> bool {
    // Todos os campos, por nome: um campo novo no `TimelineItem` não passa despercebido.
    let TimelineItem {
        img,
        cut,
        spread,
        x,
        row,
    } = a;
    let empty = |i: &slint::Image| i.size().width == 0;
    *cut == b.cut
        && *spread == b.spread
        && *row == b.row
        && x.to_bits() == b.x.to_bits()
        && (*img == b.img || (empty(img) && empty(&b.img)))
}

/// Uma vaga sem item: um corte fora da tela, à esquerda (sem imagem para subir).
fn empty_slot() -> TimelineItem {
    TimelineItem {
        cut: true,
        spread: -1,
        x: -1000.0,
        row: -1,
        ..TimelineItem::default()
    }
}

/// A largura de um item.
fn width_of(t: &TimelineItem) -> f32 {
    if t.cut { CUT } else { PHOTO }
}

/// A posição de cada item na linha inteira, na ordem: a margem e um espaço entre eles.
pub fn place(items: &mut [TimelineItem]) {
    let mut x = PADDING;
    for t in items {
        t.x = x;
        x += width_of(t) + SPACING;
    }
}

/// O primeiro `i` em `0..n` para o qual `before(i)` é falso (a lista está em ordem).
fn partition(n: usize, before: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let mid = usize::midpoint(lo, hi);
        if before(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize) -> Rc<VecModel<TimelineItem>> {
        let mut items: Vec<TimelineItem> = (0..n)
            .map(|i| TimelineItem {
                cut: i % 5 == 4,
                spread: i32::try_from(i / 5).unwrap_or(0),
                ..TimelineItem::default()
            })
            .collect();
        place(&mut items);
        Rc::new(VecModel::from(items))
    }

    /// Os itens de verdade nas vagas, em ordem de posição.
    fn shown(w: &LineWindow) -> Vec<TimelineItem> {
        let mut v: Vec<TimelineItem> = (0..w.model.row_count())
            .filter_map(|s| w.model.row_data(s))
            .filter(|t| t.spread >= 0)
            .collect();
        v.sort_by(|a, b| a.x.total_cmp(&b.x));
        v
    }

    #[test]
    fn so_os_itens_perto_do_trecho_visivel_vao_para_a_tela() {
        let all = line(750);
        let mut w = LineWindow::new();
        w.set_view(0.0, 1000.0);
        let total = w.refresh(all.as_ref());
        // 600 fotos de 46 px e 150 cortes de 14 px, com 2 px entre eles e 6 de margem.
        assert!((total - (6.0 + 600.0 * 48.0 + 150.0 * 16.0 - 2.0 + 6.0)).abs() < 0.01);
        let items = shown(&w);
        assert!(
            items.len() > 30 && items.len() < 60,
            "{} itens na tela",
            items.len()
        );
        assert!((items[0].x - 6.0).abs() < 0.01);
        // Rolou até o meio: outros itens, o mesmo tanto.
        w.set_view(15_000.0, 1000.0);
        w.refresh(all.as_ref());
        let items = shown(&w);
        assert!((15_000.0 - MARGIN - 48.0..=15_000.0 - MARGIN).contains(&items[0].x));
        assert!(items.len() < 60);
    }

    #[test]
    fn rl_b29_rolar_a_linha_troca_so_as_vagas_dos_itens_que_entraram() {
        let all = line(750);
        let mut w = LineWindow::new();
        w.set_view(3_000.0, 1_600.0);
        w.refresh(all.as_ref());
        let slots = w.model.row_count();
        let before: Vec<TimelineItem> = (0..slots).filter_map(|s| w.model.row_data(s)).collect();
        // Uma lâmina para a direita (8 fotos e um corte, ~400 px), como ao navegar.
        w.set_view(3_400.0, 1_600.0);
        w.refresh(all.as_ref());
        assert_eq!(w.model.row_count(), slots, "o número de vagas fica");
        let after: Vec<TimelineItem> = (0..slots).filter_map(|s| w.model.row_data(s)).collect();
        // Quem continua no trecho fica na mesma vaga, sem ser regravado; só mudam as vagas dos
        // itens que entraram ou saíram (uns 9 de cada lado).
        let near = |t: &TimelineItem| {
            t.spread >= 0 && t.x + width_of(t) >= 3_400.0 - MARGIN && t.x <= 5_000.0 + MARGIN
        };
        for (old, new) in before.iter().zip(&after) {
            if near(old) {
                assert!(same(old, new), "o item em {} mudou de vaga", old.x);
            }
        }
        let changed = before
            .iter()
            .zip(&after)
            .filter(|(a, b)| !same(a, b))
            .count();
        assert!(changed <= 20, "{changed} vagas trocadas de {slots}");
        // Cada item perto do trecho está numa vaga só, e na posição dele.
        let items = shown(&w);
        let expected: Vec<f32> = (0..all.row_count())
            .filter_map(|i| all.row_data(i))
            .filter(near)
            .map(|t| t.x)
            .collect();
        let got: Vec<f32> = items.iter().map(|t| t.x).collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn rl_b29_janela_mais_larga_ganha_vagas_e_mostra_tudo() {
        let all = line(750);
        let mut w = LineWindow::new();
        w.set_view(0.0, 500.0);
        w.refresh(all.as_ref());
        let small = w.model.row_count();
        w.set_view(0.0, 3_000.0);
        w.refresh(all.as_ref());
        assert!(w.model.row_count() > small);
        let items = shown(&w);
        assert!(
            items.windows(2).all(|p| p[0].x < p[1].x),
            "sem item repetido"
        );
        assert!(items.last().is_some_and(|t| t.x > 3_000.0));
    }
}
