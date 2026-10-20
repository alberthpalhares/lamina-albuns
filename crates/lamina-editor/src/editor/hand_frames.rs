//! Quadros novos, apagar e a ordem pelo editor (spec TX, regras 11 a 13). O gesto e as setas
//! ficam em `hand.rs`.

use lamina_core::{FrameContent, PageSide};

use super::Editor;
use crate::hand::{add_empty_frame, add_text, page_for, remove_frame, reorder};

impl Editor {
    /// T: texto novo na página pedida (a do mouse), selecionado (regra 11).
    pub fn new_text(&mut self, side: Option<PageSide>) -> bool {
        self.new_frame("Novo texto", side, true)
    }

    /// Lâmina → Novo quadro de foto: um quadro vazio no centro da página.
    pub fn new_photo_frame(&mut self, side: Option<PageSide>) -> bool {
        self.new_frame("Novo quadro", side, false)
    }

    fn new_frame(&mut self, label: &str, side: Option<PageSide>, text: bool) -> bool {
        self.exit_crop();
        let mut at = None;
        let done = self.change(label, |p, i| {
            let side = page_for(p, i, side);
            at = Some(if text {
                add_text(p, i, side)?
            } else {
                add_empty_frame(p, i, side)?
            });
            Ok(())
        });
        if done {
            self.selected_frame = at;
        }
        done
    }

    /// Delete: tira a foto de um quadro com foto; apaga o quadro vazio ou de texto.
    pub(super) fn delete_selected(&mut self) -> bool {
        if self.can_clear() {
            return self.clear_selected();
        }
        let Some(frame) = self.selected_frame else {
            return false;
        };
        let done = self.change("Apagar quadro", |p, i| remove_frame(p, i, frame));
        if done {
            self.selected_frame = None;
        }
        done
    }

    /// Há um quadro de foto (com ou sem foto) selecionado para mudar de ordem?
    pub(super) fn can_reorder(&self) -> bool {
        self.selected_content()
            .is_some_and(|c| !matches!(c, FrameContent::Text(_)))
    }

    pub(super) fn reorder_selected(&mut self, forward: bool) -> bool {
        let Some(frame) = self.selected_frame else {
            return false;
        };
        let mut to = frame;
        let done = self.change("Ordem dos quadros", |p, i| {
            to = reorder(p, i, frame, forward)?;
            Ok(())
        });
        if done {
            self.selected_frame = Some(to);
        }
        done
    }

    /// O conteúdo do quadro selecionado na lâmina atual.
    pub fn selected_content(&self) -> Option<&FrameContent> {
        let s = self.session.as_ref()?;
        let frame = self.selected_frame?;
        Some(
            &s.project()
                .spreads
                .get(s.current())?
                .frames
                .get(frame)?
                .content,
        )
    }
}
