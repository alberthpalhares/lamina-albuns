//! Alinhar, Igualar e Distribuir no modo organizar (spec EG, regra 9). As contas estão em
//! [`crate::arrange`]; cada comando é uma edição. Os campos da seleção ficam em `group_fields.rs`.

use lamina_core::Rect;

use super::Editor;
use crate::Command;
use crate::arrange::{Align, Equal, align, distribute, equalize};
use crate::hand::set_rect;

/// O aviso de Distribuir com menos de três quadros (seção 6).
pub const NEED_THREE: &str = "Selecione três quadros ou mais.";
/// O que cada comando faz com os retângulos (e o rótulo do histórico).
enum Arrange {
    Align(Align),
    Equal(Equal),
    Distribute(bool),
}

fn arrange_of(command: Command) -> Option<Arrange> {
    use Command as C;
    Some(match command {
        C::AlignLeft => Arrange::Align(Align::Left),
        C::AlignCenter => Arrange::Align(Align::Center),
        C::AlignRight => Arrange::Align(Align::Right),
        C::AlignTop => Arrange::Align(Align::Top),
        C::AlignMiddle => Arrange::Align(Align::Middle),
        C::AlignBottom => Arrange::Align(Align::Bottom),
        C::EqualWidth => Arrange::Equal(Equal::Width),
        C::EqualHeight => Arrange::Equal(Equal::Height),
        C::EqualSize => Arrange::Equal(Equal::Size),
        C::DistributeAcross => Arrange::Distribute(true),
        C::DistributeDown => Arrange::Distribute(false),
        _ => return None,
    })
}

impl Editor {
    /// Disponibilidade de Alinhar, Igualar e Distribuir; `None` = não é deles.
    pub(super) fn arrange_available(&self, command: Command) -> Option<bool> {
        let n = self.selected_frames().len();
        let need = match arrange_of(command)? {
            Arrange::Distribute(_) => 3,
            _ => 2,
        };
        Some(self.organizing() && n >= need)
    }

    /// Roda Alinhar, Igualar ou Distribuir; `None` = não é deles.
    pub(super) fn arrange_run(&mut self, command: Command) -> Option<bool> {
        let how = arrange_of(command)?;
        if self.arrange_available(command) != Some(true) {
            return Some(false);
        }
        let frames = self.selected_frames();
        let all = self.frames_mm();
        let rects: Vec<Rect> = frames.iter().filter_map(|f| all.get(*f).copied()).collect();
        let reference = self
            .selected_frame
            .and_then(|r| frames.iter().position(|f| *f == r))
            .unwrap_or(0);
        let (moved, label) = match how {
            Arrange::Align(a) => (align(&rects, reference, a), "Alinhar quadros"),
            Arrange::Equal(e) => (equalize(&rects, reference, e), command.label()),
            Arrange::Distribute(across) => (distribute(&rects, across), "Distribuir quadros"),
        };
        let Some(moved) = moved.filter(|m| *m != rects) else {
            return Some(false);
        };
        Some(self.change(label, |p, i| {
            frames
                .iter()
                .zip(&moved)
                .try_for_each(|(f, r)| set_rect(p, i, *f, *r))
        }))
    }

    /// "Selecione três quadros ou mais." enquanto Distribuir não vale no modo (vazio quando vale).
    pub fn distribute_hint(&self) -> &'static str {
        if self.organizing() && self.selected_frames().len() < 3 {
            NEED_THREE
        } else {
            ""
        }
    }
}
