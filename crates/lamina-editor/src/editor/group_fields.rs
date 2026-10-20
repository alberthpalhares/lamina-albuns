//! Os campos da seleção no modo organizar (spec EG, seção 3): X e Y contados do corte, largura e
//! altura da caixa do grupo, com "Manter proporção". Cada campo digitado é uma edição.

use lamina_core::{FrameContent, Mm, Rect};

use super::Editor;
use crate::group::{clamp_shift, group_box, scaled_text, shifted};
use crate::hand::{MIN_FRAME_MM, set_rect};

/// O número inválido nos campos da seleção (seção 6).
pub const GROUP_NOT_A_NUMBER: &str = "Use um número, como 12,5.";

/// Um campo da seleção.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    X,
    Y,
    Width,
    Height,
}

impl Editor {
    /// A caixa do grupo selecionado em mm, com X e Y contados do corte (o zero das réguas).
    pub fn group_mm(&self) -> Option<Rect> {
        let s = self.session.as_ref()?;
        let all = self.frames_mm();
        let rects: Vec<Rect> = self
            .selected_frames()
            .iter()
            .filter_map(|f| all.get(*f).copied())
            .collect();
        let b = group_box(&rects)?;
        let trim = s.project().spec.trim_rect();
        Some(Rect::new(
            Mm(b.x.get() - trim.x.get()),
            Mm(b.y.get() - trim.y.get()),
            b.width,
            b.height,
        ))
    }

    /// O texto dos campos X, Y, largura e altura na unidade da tela (vazios sem seleção).
    pub fn group_fields(&self) -> [String; 4] {
        let unit = self.strips.strips.unit;
        self.group_mm().map_or(Default::default(), |b| {
            [b.x, b.y, b.width, b.height].map(|v| unit.show(v.get()))
        })
    }

    /// Um campo da seleção digitado: X e Y movem o grupo, largura e altura o ampliam a partir do
    /// canto de cima à esquerda (com `keep`, nos dois eixos). Texto inválido: nada muda.
    pub fn set_group_field(
        &mut self,
        field: Field,
        text: &str,
        keep: bool,
    ) -> Result<bool, &'static str> {
        let value = self
            .strips
            .strips
            .unit
            .read(text)
            .map_err(|_| GROUP_NOT_A_NUMBER)?;
        let Some(b) = self.group_mm() else {
            return Ok(false);
        };
        let (sx, sy) = match field {
            Field::X => return Ok(self.move_group_to(value - b.x.get(), 0.0)),
            Field::Y => return Ok(self.move_group_to(0.0, value - b.y.get())),
            Field::Width => {
                let s = value / b.width.get();
                (s, if keep { s } else { 1.0 })
            }
            Field::Height => {
                let s = value / b.height.get();
                (if keep { s } else { 1.0 }, s)
            }
        };
        Ok(self.scale_group(sx, sy))
    }

    fn move_group_to(&mut self, dx: f64, dy: f64) -> bool {
        let (rects, frames) = self.selection_rects();
        let (Some(s), Some(b)) = (&self.session, group_box(&rects)) else {
            return false;
        };
        let (dx, dy) = clamp_shift(b, s.project().spec.trim_rect(), dx, dy);
        if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
            return false;
        }
        let moved = shifted(&rects, dx, dy);
        let label = counted("Mover quadro", "Mover", frames.len());
        self.change(&label, |p, i| {
            frames
                .iter()
                .zip(&moved)
                .try_for_each(|(f, r)| set_rect(p, i, *f, *r))
        })
    }

    /// Amplia o grupo `sx` × `sy` a partir do canto de cima à esquerda, parando no corte e no
    /// mínimo de cada quadro; o texto escala junto (regra 5).
    fn scale_group(&mut self, sx: f64, sy: f64) -> bool {
        let (rects, frames) = self.selection_rects();
        let (Some(s), Some(b)) = (&self.session, group_box(&rects)) else {
            return false;
        };
        let trim = s.project().spec.trim_rect();
        let small = |w: fn(&Rect) -> f64| rects.iter().map(w).fold(f64::MAX, f64::min);
        let fit = |v: f64, lo: f64, hi: f64| v.clamp(lo.min(hi), hi);
        let sx = fit(
            sx,
            MIN_FRAME_MM / small(|r| r.width.get()),
            (trim.right().get() - b.x.get()) / b.width.get(),
        );
        let sy = fit(
            sy,
            MIN_FRAME_MM / small(|r| r.height.get()),
            (trim.bottom().get() - b.y.get()) / b.height.get(),
        );
        if (sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9 {
            return false;
        }
        let (bx, by) = (b.x.get(), b.y.get());
        let scaled: Vec<Rect> = rects
            .iter()
            .map(|r| {
                Rect::new(
                    Mm(bx + (r.x.get() - bx) * sx),
                    Mm(by + (r.y.get() - by) * sy),
                    Mm(r.width.get() * sx),
                    Mm(r.height.get() * sy),
                )
            })
            .collect();
        let label = counted("Redimensionar quadro", "Redimensionar", frames.len());
        self.change(&label, |p, i| {
            for (f, r) in frames.iter().zip(&scaled) {
                set_rect(p, i, *f, *r)?;
                if let Some(FrameContent::Text(t)) = p
                    .spreads
                    .get_mut(i)
                    .and_then(|s| s.frames.get_mut(*f))
                    .map(|f| &mut f.content)
                {
                    t.size_pt = scaled_text(t.size_pt, sx, sy);
                }
            }
            Ok(())
        })
    }

    /// Os retângulos (mm) e os índices dos quadros selecionados.
    fn selection_rects(&self) -> (Vec<Rect>, Vec<usize>) {
        let frames = self.selected_frames();
        let all = self.frames_mm();
        let rects = frames.iter().filter_map(|f| all.get(*f).copied()).collect();
        (rects, frames)
    }
}

/// "Mover quadro" com um; "Mover 3 quadros" com mais.
fn counted(one: &str, many: &str, n: usize) -> String {
    if n == 1 {
        one.to_owned()
    } else {
        format!("{many} {n} quadros")
    }
}
