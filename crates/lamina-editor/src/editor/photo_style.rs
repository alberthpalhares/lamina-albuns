//! Opacidade, borda, cor da borda e cantos na seção Foto (spec OP, regras 7, 8 e 11): valem para
//! os quadros de foto selecionados (no modo organizar, todos eles, numa edição só), e a seção
//! mostra os valores do primeiro clicado e o tamanho do quadro.

use lamina_core::{Border, Mm, Opacity, Project, Rgb};

use super::Editor;
use crate::forms::{BORDER_FIRST, LookField};

/// O que a seção Foto mostra da aparência do quadro.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoStyle {
    /// Do primeiro quadro clicado.
    pub opacity: Opacity,
    pub border: Border,
    pub radius: Mm,
    /// Quantos quadros de foto a mudança atinge.
    pub frames: usize,
    /// "Quadro de 12,4 × 8,3 cm", com um quadro só (R0-22); vazio com vários.
    pub size: String,
}

impl Editor {
    /// A aparência dos quadros de foto selecionados; `None` sem nenhum.
    pub fn photo_style(&self) -> Option<PhotoStyle> {
        let (frames, _) = self.photo_targets();
        let s = self.session.as_ref()?;
        let project = s.project();
        let first = project
            .spreads
            .get(s.current())?
            .frames
            .get(*frames.first()?)?;
        let size = if frames.len() == 1 {
            let r = project.frame_rect(first);
            let unit = self.strips.strips.unit;
            format!(
                "Quadro de {} × {} {}",
                unit.show(r.width.get()),
                unit.show(r.height.get()),
                unit.label()
            )
        } else {
            String::new()
        };
        Some(PhotoStyle {
            opacity: first.opacity,
            border: first.border,
            radius: first.radius,
            frames: frames.len(),
            size,
        })
    }

    /// "Vale para os 3 quadros selecionados" quando a mudança atinge mais de um quadro.
    pub fn photo_style_note(&self) -> String {
        match self.photo_style().map(|s| s.frames) {
            Some(n) if n > 1 => format!("Vale para os {n} quadros selecionados"),
            _ => String::new(),
        }
    }

    /// O controle da opacidade (0 a 100). Arrastar emenda numa edição só; soltar fecha
    /// ([`Editor::end_adjust`]). O mesmo valor de antes não cria edição.
    pub fn set_photo_opacity(&mut self, percent: u8) -> bool {
        let Some(opacity) = Opacity::new(percent) else {
            return false;
        };
        let (frames, _) = self.photo_targets();
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let index = s.current();
        let ids = frame_ids(s.project(), index, &frames);
        if !changes(s.project(), index, &frames, |f| f.opacity != opacity) {
            return false;
        }
        let key = format!("opacidade:{index}:{ids:?}");
        s.edit_merging("Opacidade da foto", &key, |p| {
            each(p, index, &frames, |f| f.opacity = opacity);
            Ok::<(), crate::EditorError>(())
        })
        .is_ok()
    }

    /// Um clique em − ou + da borda ou dos cantos da seção Foto: uma edição por clique (AP).
    pub fn step_photo_style(&mut self, field: LookField, delta: i32) -> bool {
        let Some(style) = self.photo_style() else {
            return false;
        };
        let (step, max) = field.step_and_max();
        let now = match field {
            LookField::Border => style.border.width.get(),
            LookField::Radius => style.radius.get(),
            LookField::Gap => return false,
        };
        let value = Mm((now + f64::from(delta) * step).clamp(0.0, max));
        match field {
            LookField::Border => self.restyle("Mudar borda", |f| {
                f.border = Border {
                    width: value,
                    color: style.border.color,
                };
            }),
            _ => self.restyle("Mudar cantos", |f| f.radius = value),
        }
    }

    /// A cor da borda na seção Foto: sem espessura ainda, a borda ganha [`BORDER_FIRST`].
    pub fn pick_photo_border_color(&mut self, color: Rgb) -> bool {
        let Some(style) = self.photo_style() else {
            return false;
        };
        let width = if style.border.width.get() > 0.0 {
            style.border.width
        } else {
            Mm(BORDER_FIRST)
        };
        self.restyle("Mudar borda", |f| f.border = Border { width, color })
    }

    /// Muda todos os quadros de foto selecionados numa edição; nada muda, nada entra.
    fn restyle(&mut self, label: &str, change: impl Fn(&mut lamina_core::Frame)) -> bool {
        let (frames, _) = self.photo_targets();
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let index = s.current();
        let differs = |f: &lamina_core::Frame| {
            let mut after = f.clone();
            change(&mut after);
            after != *f
        };
        if !changes(s.project(), index, &frames, differs) {
            return false;
        }
        s.edit(label, |p| {
            each(p, index, &frames, &change);
            Ok::<(), crate::EditorError>(())
        })
        .is_ok()
    }
}

/// Algum dos quadros `frames` da lâmina `index` mudaria?
fn changes(
    p: &Project,
    index: usize,
    frames: &[usize],
    differs: impl Fn(&lamina_core::Frame) -> bool,
) -> bool {
    p.spreads
        .get(index)
        .is_some_and(|s| frames.iter().filter_map(|i| s.frames.get(*i)).any(differs))
}

/// Aplica `change` nos quadros `frames` da lâmina `index`.
pub(super) fn each(
    p: &mut Project,
    index: usize,
    frames: &[usize],
    change: impl Fn(&mut lamina_core::Frame),
) {
    if let Some(s) = p.spreads.get_mut(index) {
        for i in frames {
            if let Some(f) = s.frames.get_mut(*i) {
                change(f);
            }
        }
    }
}

fn frame_ids(p: &Project, index: usize, frames: &[usize]) -> Vec<lamina_core::FrameId> {
    p.spreads.get(index).map_or_else(Vec::new, |s| {
        frames
            .iter()
            .filter_map(|i| s.frames.get(*i).map(|f| f.id))
            .collect()
    })
}
