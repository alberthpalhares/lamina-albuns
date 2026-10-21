//! A linha do tempo virtualizada (spec RL, regra 22; spec PF, seção 10: "virtualizar a linha do
//! tempo → RL"): a lista inteira continua no `Synced` dela (`lists.line`), com a posição de cada
//! item, e a tela recebe só os itens perto do trecho visível. Com 600 fotos, a linha inteira na
//! tela custava um quadro de cada três na navegação (medido no G1).

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

/// O que a tela mostra da linha do tempo.
pub struct LineWindow {
    model: Rc<VecModel<TimelineItem>>,
    start: f32,
    width: f32,
    shown: std::ops::Range<usize>,
}

impl LineWindow {
    pub fn new() -> Self {
        LineWindow {
            model: Rc::new(VecModel::default()),
            start: 0.0,
            width: 0.0,
            shown: 0..0,
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
        if range == self.shown {
            for (k, i) in range.clone().enumerate() {
                let (old, new) = (self.model.row_data(k), all.row_data(i));
                if let Some(new) = new
                    && old.as_ref() != Some(&new)
                {
                    self.model.set_row_data(k, new);
                }
            }
        } else {
            let rows: Vec<TimelineItem> = range.clone().filter_map(|i| all.row_data(i)).collect();
            self.model.set_vec(rows);
            self.shown = range;
        }
        total
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

    #[test]
    fn so_os_itens_perto_do_trecho_visivel_vao_para_a_tela() {
        let all = line(750);
        let mut w = LineWindow::new();
        w.set_view(0.0, 1000.0);
        let total = w.refresh(all.as_ref());
        // 600 fotos de 46 px e 150 cortes de 14 px, com 2 px entre eles e 6 de margem.
        assert!((total - (6.0 + 600.0 * 48.0 + 150.0 * 16.0 - 2.0 + 6.0)).abs() < 0.01);
        let shown = w.model.row_count();
        assert!(shown > 30 && shown < 60, "{shown} itens na tela");
        assert!((w.model.row_data(0).unwrap().x - 6.0).abs() < 0.01);
        // Rolou até o meio: outros itens, o mesmo tanto.
        w.set_view(15_000.0, 1000.0);
        w.refresh(all.as_ref());
        let first = w.model.row_data(0).unwrap().x;
        assert!((15_000.0 - MARGIN - 48.0..=15_000.0 - MARGIN).contains(&first));
        assert!(w.model.row_count() < 60);
    }
}
