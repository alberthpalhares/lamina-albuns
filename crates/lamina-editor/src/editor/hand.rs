//! Quadros à mão pelo editor (spec TX): o gesto de mover ou redimensionar com o ímã, Alt+setas,
//! quadros novos, apagar e a ordem. As contas estão em [`crate::hand`] e [`crate::snap`].

use lamina_core::{FrameContent, Mm, PageSide, Rect};

use super::Editor;
use crate::hand::{
    MIN_FRAME_MM, add_empty_frame, add_text, fit_rect, frame_mm, page_for, remove_frame, reorder,
    set_rect,
};
use crate::keys::{Key, Shortcut};
use crate::snap::{Grip, SnapLine, dragged, guides, snap};

/// O gesto em andamento: o quadro, a alça, o retângulo do começo e o de agora (em mm).
#[derive(Debug, Clone)]
pub(super) struct Gesture {
    frame: usize,
    grip: Grip,
    start: Rect,
    now: Rect,
    lines: Vec<SnapLine>,
}

impl Editor {
    /// Começa a mover ou redimensionar o quadro `frame` da lâmina atual; ele fica selecionado.
    pub fn frame_grab(&mut self, frame: usize, grip: Grip) -> bool {
        self.exit_crop();
        let Some(s) = &self.session else {
            return false;
        };
        let Some(start) = frame_mm(s.project(), s.current(), frame) else {
            return false;
        };
        self.selected_frame = Some(frame);
        self.gesture = Some(Gesture {
            frame,
            grip,
            start,
            now: start,
            lines: Vec::new(),
        });
        true
    }

    /// Arrasto de `dx`, `dy` mm desde o começo do gesto. Com `magnet`, encaixa nas guias a menos
    /// de `tolerance` mm (8 pixels da tela, convertidos pela janela).
    pub fn frame_drag(&mut self, dx: f64, dy: f64, magnet: bool, tolerance: f64) -> bool {
        let (Some(s), Some(g)) = (&self.session, &self.gesture) else {
            return false;
        };
        let moved = dragged(g.start, g.grip, dx, dy, MIN_FRAME_MM);
        let (rect, lines) = if magnet {
            snap(
                moved,
                g.grip,
                &guides(s.project(), s.current(), g.frame),
                tolerance,
            )
        } else {
            (moved, Vec::new())
        };
        // O que a tela mostra já é o que vai ficar: preso ao corte e ao mínimo.
        let fitted = fit_rect(s.project(), rect)
            .map(|r| r.to_absolute(&s.project().spec.trim_rect()))
            .unwrap_or(rect);
        if let Some(g) = self.gesture.as_mut() {
            g.now = fitted;
            g.lines = lines;
        }
        true
    }

    /// Solta: grava o gesto como uma edição (regra 10). Sem movimento, nada muda.
    pub fn frame_release(&mut self) -> bool {
        let Some(g) = self.gesture.take() else {
            return false;
        };
        if g.now == g.start {
            return false;
        }
        let label = match g.grip {
            Grip::Move => "Mover quadro",
            Grip::Resize { .. } => "Redimensionar quadro",
        };
        self.change(label, |p, i| set_rect(p, i, g.frame, g.now))
    }

    /// Esc no meio do gesto: o quadro volta.
    pub fn frame_cancel(&mut self) {
        self.gesture = None;
    }

    /// O quadro do gesto e onde ele está agora (mm), para a tela desenhar.
    pub fn gesture_rect(&self) -> Option<(usize, Rect)> {
        self.gesture.as_ref().map(|g| (g.frame, g.now))
    }

    /// As guias que pegaram no gesto, para as linhas da tela.
    pub fn snap_lines(&self) -> &[SnapLine] {
        self.gesture.as_ref().map_or(&[], |g| g.lines.as_slice())
    }

    /// Alt+setas move o quadro selecionado 1 mm; com Shift, 10 mm. Seguidas no mesmo quadro
    /// viram uma edição (regra 10). `false` = não era Alt+seta ou não há quadro.
    pub fn nudge_key(&mut self, key: Shortcut) -> bool {
        if !key.alt || key.ctrl {
            return false;
        }
        let step = if key.shift { 10.0 } else { 1.0 };
        let (dx, dy) = match key.key {
            Key::Left => (-step, 0.0),
            Key::Right => (step, 0.0),
            Key::Up => (0.0, -step),
            Key::Down => (0.0, step),
            _ => return false,
        };
        let (Some(frame), Some(s)) = (self.selected_frame, self.session.as_mut()) else {
            return false;
        };
        let index = s.current();
        let Some(r) = frame_mm(s.project(), index, frame) else {
            return false;
        };
        let to = Rect::new(Mm(r.x.get() + dx), Mm(r.y.get() + dy), r.width, r.height);
        let key = format!("mover:{index}:{frame}");
        s.edit_merging("Mover quadro", &key, |p| set_rect(p, index, frame, to))
            .is_ok()
    }

    /// Põe o quadro selecionado nas medidas digitadas na aba Quadro (mm, regra 12).
    pub fn set_frame_mm(&mut self, rect: Rect) -> bool {
        let Some(frame) = self.selected_frame else {
            return false;
        };
        self.change("Mover quadro", |p, i| set_rect(p, i, frame, rect))
    }

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
