//! O modo organizar a lâmina (spec EG): entrar e sair, a seleção de vários quadros (guardada pelos
//! ids, para sobreviver ao desfazer), o laço, as setas que movem o grupo e o zoom da lâmina. O
//! modo, a seleção e o zoom não vão para o arquivo (regra 3).

use lamina_core::{FrameId, Mm, Rect, SpreadId};

use super::Editor;
use crate::commands::Mode;
use crate::hand::{frame_mm, set_rect};
use crate::view::{Area, spread_view};

/// Os zooms da lâmina no modo: "Ajustar", 2× e 4× (regra 14).
const ZOOMS: [f64; 3] = [1.0, 2.0, 4.0];

/// O estado do modo organizar.
#[derive(Debug, Default)]
pub(super) struct Organizing {
    on: bool,
    /// A lâmina da seleção e os quadros, na ordem dos cliques (o primeiro é a referência).
    spread: Option<SpreadId>,
    ids: Vec<FrameId>,
    zoom: usize,
}

/// O que o duplo clique na lâmina fez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleClick {
    /// Fora dos quadros: entrou no modo organizar.
    Organize,
    /// Numa foto: abriu o recorte.
    Crop,
    /// Num texto: o cursor vai para o campo do texto.
    Text,
    Nothing,
}

/// O painel da direita (EG-B23): as abas de sempre ou o painel do modo organizar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePanel {
    Tabs,
    Organize,
}

impl Editor {
    pub fn organizing(&self) -> bool {
        self.organize.on
    }

    pub fn mode(&self) -> Mode {
        if self.organize.on {
            Mode::Organize
        } else {
            Mode::Normal
        }
    }

    pub fn side_panel(&self) -> SidePanel {
        if self.organize.on {
            SidePanel::Organize
        } else {
            SidePanel::Tabs
        }
    }

    /// Entra ou sai do modo. Sair desmarca os quadros e volta ao zoom "Ajustar".
    pub(super) fn set_organizing(&mut self, on: bool) -> bool {
        if self.session.is_none() || self.organize.on == on {
            return false;
        }
        self.organize = Organizing {
            on,
            ..Organizing::default()
        };
        if !on {
            self.selected_frame = None;
        }
        true
    }

    /// Duplo clique na lâmina grande: fora dos quadros (`None`) entra no modo; numa foto abre o
    /// recorte; num texto, o campo do texto (spec EG, regra de entrada; EG-B2).
    pub fn double_click(&mut self, frame: Option<usize>) -> DoubleClick {
        let Some(frame) = frame else {
            self.set_organizing(true);
            return if self.organize.on {
                DoubleClick::Organize
            } else {
                DoubleClick::Nothing
            };
        };
        self.select_frame(Some(frame));
        if self.selected_text().is_some() {
            DoubleClick::Text
        } else if self.enter_crop(frame) {
            DoubleClick::Crop
        } else {
            DoubleClick::Nothing
        }
    }

    /// Os ids dos quadros da lâmina atual, na ordem dela.
    fn current_ids(&self) -> Option<(SpreadId, Vec<FrameId>)> {
        let s = self.session.as_ref()?;
        let spread = s.project().spreads.get(s.current())?;
        Some((spread.id, spread.frames.iter().map(|f| f.id).collect()))
    }

    /// Clique num quadro. No modo, Ctrl ou Shift soma ou tira; fora dele, só seleciona um.
    pub fn organize_click(&mut self, frame: usize, ctrl: bool, shift: bool) {
        if !self.organize.on {
            self.select_frame(Some(frame));
            return;
        }
        let Some((spread, ids)) = self.current_ids() else {
            return;
        };
        let Some(&id) = ids.get(frame) else {
            return;
        };
        if self.organize.spread != Some(spread) {
            self.organize.ids.clear();
            self.organize.spread = Some(spread);
        }
        if ctrl || shift {
            if let Some(at) = self.organize.ids.iter().position(|i| *i == id) {
                self.organize.ids.remove(at);
            } else {
                self.organize.ids.push(id);
            }
        } else {
            self.organize.ids = vec![id];
        }
        self.sync_reference();
    }

    /// O quadro de referência (o primeiro clicado) vira o quadro selecionado de sempre.
    fn sync_reference(&mut self) {
        let reference = self.organize.ids.first().copied();
        self.selected_frame = self
            .current_ids()
            .and_then(|(_, ids)| reference.and_then(|r| ids.iter().position(|i| *i == r)));
    }

    /// Os quadros selecionados da lâmina atual, na ordem da lâmina. Fora do modo, o quadro
    /// selecionado de sempre.
    pub fn selected_frames(&self) -> Vec<usize> {
        if !self.organize.on {
            return self.selected_frame.into_iter().collect();
        }
        let Some((spread, ids)) = self.current_ids() else {
            return Vec::new();
        };
        if self.organize.spread != Some(spread) {
            return Vec::new();
        }
        (0..ids.len())
            .filter(|i| self.organize.ids.contains(&ids[*i]))
            .collect()
    }

    /// "1 quadro", "3 quadros" (nada selecionado: vazio).
    pub fn selection_text(&self) -> String {
        match self.selected_frames().len() {
            0 => String::new(),
            1 => "1 quadro".to_owned(),
            n => format!("{n} quadros"),
        }
    }

    pub(super) fn select_all_frames(&mut self) -> bool {
        let Some((spread, ids)) = self.current_ids() else {
            return false;
        };
        self.organize.spread = Some(spread);
        self.organize.ids = ids;
        self.sync_reference();
        !self.organize.ids.is_empty()
    }

    /// Desmarca os quadros do modo; `true` se havia algum.
    pub(super) fn clear_frames(&mut self) -> bool {
        let had = !self.selected_frames().is_empty();
        self.organize.ids.clear();
        self.selected_frame = None;
        had
    }

    /// O laço de `a` a `b` (frações da largura da lâmina): pega os quadros inteiros dentro dele;
    /// com `add`, soma à seleção.
    pub fn lasso(&mut self, a: (f64, f64), b: (f64, f64), add: bool) {
        let (Some((spread, ids)), Some(s)) = (self.current_ids(), self.session.as_ref()) else {
            return;
        };
        let canvas = s.project().spec.canvas_rect();
        let area = Area {
            width: 1.0,
            height: canvas.height.get() / canvas.width.get(),
            margin: 0.0,
        };
        let Some(view) = spread_view(s.project(), s.current(), area, &[]) else {
            return;
        };
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let inside = view.frames.iter().enumerate().filter(|(_, f)| {
            let r = f.rect;
            r.x >= x0 && r.y >= y0 && r.x + r.width <= x1 && r.y + r.height <= y1
        });
        let caught: Vec<FrameId> = inside.map(|(i, _)| ids[i]).collect();
        if !add || self.organize.spread != Some(spread) {
            self.organize.ids.clear();
        }
        self.organize.spread = Some(spread);
        for id in caught {
            if !self.organize.ids.contains(&id) {
                self.organize.ids.push(id);
            }
        }
        self.sync_reference();
    }

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
