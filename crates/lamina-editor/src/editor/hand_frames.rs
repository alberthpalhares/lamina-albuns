//! Quadros novos, apagar e a ordem pelo editor (spec TX, regras 11 a 13). O gesto e as setas
//! ficam em `hand.rs`.

use lamina_core::{FrameContent, Mm, PageSide, Rect};

use super::Editor;
use crate::hand::{add_empty_frame, add_frame_at, add_text, page_for, remove_frame, reorder};

impl Editor {
    /// T: texto novo na página pedida (a do mouse), selecionado (regra 11).
    /// O painel aparece com o texto novo, se estava escondido (spec JE, regra 5).
    pub fn new_text(&mut self, side: Option<PageSide>) -> bool {
        let done = self.new_frame("Novo texto", side, true);
        if done {
            self.ask_panel_for_selected();
        }
        done
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

    /// A paleta do modo organizar (spec EG, seção 3): arrastar com a ferramenta Quadro ou Texto
    /// desenha o quadro de `a` a `b` (frações da largura da lâmina com a sangria, como o laço). O
    /// quadro novo fica selecionado.
    pub fn draw_frame(&mut self, text: bool, a: (f64, f64), b: (f64, f64)) -> bool {
        let Some(s) = &self.session else {
            return false;
        };
        let canvas = s.project().spec.canvas_rect();
        let w = canvas.width.get();
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let rect = Rect::new(
            Mm(canvas.x.get() + x0 * w),
            Mm(canvas.y.get() + y0 * w),
            Mm((x1 - x0) * w),
            Mm((y1 - y0) * w),
        );
        let label = if text { "Novo texto" } else { "Novo quadro" };
        let mut at = None;
        let done = self.change(label, |p, i| {
            at = Some(add_frame_at(p, i, rect, text)?);
            Ok(())
        });
        let id = self.session.as_ref().and_then(|s| {
            let spread = s.project().spreads.get(s.current())?;
            Some(spread.frames.get(at?)?.id)
        });
        if let (true, Some(id)) = (done, id) {
            self.select_ids(vec![id]);
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
