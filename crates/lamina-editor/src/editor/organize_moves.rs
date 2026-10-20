//! O modo organizar, parte 2 (spec EG, regras 4, 10 e 14): os quadros em mm, as setas que movem
//! o grupo e o zoom da lâmina. A entrada no modo e a seleção ficam em `organize.rs`.

use lamina_core::{Mm, Rect};

use super::Editor;
use super::organize::ZOOMS;
use crate::hand::{frame_mm, set_rect};

impl Editor {
    /// Os quadros da lâmina atual em mm.
    pub fn frames_mm(&self) -> Vec<Rect> {
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        let count = s
            .project()
            .spreads
            .get(s.current())
            .map_or(0, |sp| sp.frames.len());
        (0..count)
            .filter_map(|f| frame_mm(s.project(), s.current(), f))
            .collect()
    }

    /// Setas no modo: o grupo anda `dx`, `dy` mm e para no corte sem deformar (regras 4 e 10);
    /// setas seguidas no mesmo grupo viram uma edição só.
    pub(super) fn nudge_group(&mut self, dx: f64, dy: f64) -> bool {
        let frames = self.selected_frames();
        let all = self.frames_mm();
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let rects: Vec<Rect> = frames.iter().filter_map(|f| all.get(*f).copied()).collect();
        let Some((dx, dy)) = clamp_move(&rects, s.project().spec.trim_rect(), dx, dy) else {
            return false;
        };
        let index = s.current();
        let label = match frames.len() {
            1 => "Mover quadro".to_owned(),
            n => format!("Mover {n} quadros"),
        };
        let key = format!("grupo:{index}:{frames:?}");
        s.edit_merging(&label, &key, |p| {
            for (f, r) in frames.iter().zip(&rects) {
                let to = Rect::new(Mm(r.x.get() + dx), Mm(r.y.get() + dy), r.width, r.height);
                set_rect(p, index, *f, to)?;
            }
            Ok::<(), crate::EditorError>(())
        })
        .is_ok()
    }

    /// O zoom da lâmina no modo (1 = "Ajustar").
    pub fn view_zoom(&self) -> f64 {
        ZOOMS[self.organize.zoom.min(ZOOMS.len() - 1)]
    }

    /// + e −: o próximo zoom para cima ou para baixo; `None` volta a "Ajustar".
    pub(super) fn step_zoom(&mut self, up: Option<bool>) -> bool {
        let before = self.organize.zoom;
        self.organize.zoom = match up {
            Some(true) => (before + 1).min(ZOOMS.len() - 1),
            Some(false) => before.saturating_sub(1),
            None => 0,
        };
        self.organize.zoom != before
    }
}

/// O deslocamento que mantém a caixa dos retângulos dentro do corte; `None` se nada anda.
fn clamp_move(rects: &[Rect], trim: Rect, dx: f64, dy: f64) -> Option<(f64, f64)> {
    let left = rects.iter().map(|r| r.x.get()).reduce(f64::min)?;
    let top = rects.iter().map(|r| r.y.get()).reduce(f64::min)?;
    let right = rects.iter().map(|r| r.right().get()).reduce(f64::max)?;
    let bottom = rects.iter().map(|r| r.bottom().get()).reduce(f64::max)?;
    // Uma caixa maior que o corte não anda (não acontece: os quadros ficam dentro do corte).
    let fit = |v: f64, lo: f64, hi: f64| if lo > hi { 0.0 } else { v.clamp(lo, hi) };
    let dx = fit(dx, trim.x.get() - left, trim.right().get() - right);
    let dy = fit(dy, trim.y.get() - top, trim.bottom().get() - bottom);
    (dx.abs() > 1e-9 || dy.abs() > 1e-9).then_some((dx, dy))
}
