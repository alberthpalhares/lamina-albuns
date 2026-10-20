//! Mover e ampliar o grupo pelo mouse no modo organizar (spec EG, regras 4 a 6 e 10): segurar o
//! corpo de um quadro selecionado ou uma alça da caixa do grupo, arrastar e soltar vira uma
//! edição. As contas estão em [`crate::group`].

use lamina_core::{FrameContent, Project, Rect};

use super::Editor;
use crate::group::{clamp_shift, group_box, scaled, scaled_text, shifted};
use crate::hand::{MIN_FRAME_MM, frame_mm, set_rect};
use crate::snap::{Grip, SnapLine, guides_without, snap};

/// O gesto do grupo: os quadros, a alça, os retângulos do começo e os de agora (mm), os corpos de
/// texto do começo e os fatores de agora.
#[derive(Debug, Clone)]
pub(super) struct GroupGesture {
    frames: Vec<usize>,
    grip: Grip,
    starts: Vec<Rect>,
    now: Vec<Rect>,
    text_pt: Vec<Option<f64>>,
    scale: (f64, f64),
    pub(super) lines: Vec<SnapLine>,
}

fn text_pt(project: &Project, index: usize, frame: usize) -> Option<f64> {
    match &project.spreads.get(index)?.frames.get(frame)?.content {
        FrameContent::Text(t) => Some(t.size_pt),
        _ => None,
    }
}

impl Editor {
    /// Segura o grupo pelo quadro `frame` (que entra na seleção, sozinho, se estava fora) com a
    /// alça `grip`. `true` = o gesto começou.
    pub fn group_grab(&mut self, frame: usize, grip: Grip) -> bool {
        self.exit_crop();
        if !self.selected_frames().contains(&frame) {
            self.organize_click(frame, false, false);
        }
        let frames = self.selected_frames();
        let all = self.frames_mm();
        let Some(s) = &self.session else {
            return false;
        };
        let (project, index) = (s.project(), s.current());
        let starts: Vec<Rect> = frames
            .iter()
            .filter_map(|f| frame_mm(project, index, *f))
            .collect();
        if starts.is_empty() || starts.len() != frames.len() || all.is_empty() {
            return false;
        }
        let text_pt = frames.iter().map(|f| text_pt(project, index, *f)).collect();
        self.group = Some(GroupGesture {
            frames,
            grip,
            now: starts.clone(),
            starts,
            text_pt,
            scale: (1.0, 1.0),
            lines: Vec::new(),
        });
        true
    }

    /// Arrasto de `dx`, `dy` mm desde o começo. Mover: a caixa encaixa nas guias (sem os quadros do
    /// grupo; `magnet` falso com Alt) e para no corte. Alça: amplia o grupo; `free` (Shift) deforma.
    pub fn group_drag(
        &mut self,
        (dx, dy): (f64, f64),
        magnet: bool,
        tolerance: f64,
        free: bool,
    ) -> bool {
        let (Some(s), Some(g)) = (&self.session, &self.group) else {
            return false;
        };
        let (project, index) = (s.project(), s.current());
        let trim = project.spec.trim_rect();
        let Some(start) = group_box(&g.starts) else {
            return false;
        };
        let (now, scale, lines) = match g.grip {
            Grip::Move => {
                let moved = shifted(&[start], dx, dy)[0];
                let (target, lines) = if magnet {
                    snap(
                        moved,
                        Grip::Move,
                        &guides_without(project, index, &g.frames),
                        tolerance,
                    )
                } else {
                    (moved, Vec::new())
                };
                let wanted = (
                    target.x.get() - start.x.get(),
                    target.y.get() - start.y.get(),
                );
                let (dx, dy) = clamp_shift(start, trim, wanted.0, wanted.1);
                (shifted(&g.starts, dx, dy), (1.0, 1.0), lines)
            }
            grip => {
                let (rects, sx, sy) = scaled(&g.starts, grip, (dx, dy), free, (trim, MIN_FRAME_MM));
                (rects, (sx, sy), Vec::new())
            }
        };
        if let Some(g) = self.group.as_mut() {
            g.now = now;
            g.scale = scale;
            g.lines = lines;
        }
        true
    }

    /// Solta: grava o gesto como uma edição ("Mover 3 quadros", "Redimensionar 3 quadros").
    pub fn group_release(&mut self) -> bool {
        let Some(g) = self.group.take() else {
            return false;
        };
        if g.now == g.starts {
            return false;
        }
        let n = g.frames.len();
        let (one, many) = match g.grip {
            Grip::Move => ("Mover quadro", "Mover"),
            Grip::Resize { .. } => ("Redimensionar quadro", "Redimensionar"),
        };
        let label = if n == 1 {
            one.to_owned()
        } else {
            format!("{many} {n} quadros")
        };
        self.change(&label, |p, i| apply_group(p, i, &g))
    }

    /// O gesto do grupo está em curso.
    pub fn group_moving(&self) -> bool {
        self.group.is_some()
    }

    /// O projeto com o gesto do grupo aplicado, para a tela desenhar no meio do gesto.
    pub(super) fn apply_group_preview(&self, project: &mut Project, index: usize) {
        if let Some(g) = &self.group {
            // Os retângulos do gesto já estão presos ao corte; erro só deixa como estava.
            let _ = apply_group(project, index, g);
        }
    }
}

/// Os retângulos e os corpos de texto do gesto no projeto.
fn apply_group(
    project: &mut Project,
    index: usize,
    g: &GroupGesture,
) -> Result<(), crate::EditorError> {
    for (k, (f, r)) in g.frames.iter().zip(&g.now).enumerate() {
        set_rect(project, index, *f, *r)?;
        let Some(start) = g.text_pt.get(k).copied().flatten() else {
            continue;
        };
        if let Some(FrameContent::Text(t)) = project
            .spreads
            .get_mut(index)
            .and_then(|s| s.frames.get_mut(*f))
            .map(|f| &mut f.content)
        {
            t.size_pt = scaled_text(start, g.scale.0, g.scale.1);
        }
    }
    Ok(())
}
