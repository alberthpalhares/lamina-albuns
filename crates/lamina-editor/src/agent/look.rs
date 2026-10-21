//! Aparência pela IA (spec IA, `aparencia`; spec AP): espaço entre fotos, fundo, borda e cantos
//! numa edição só, e com `todas` a aparência da lâmina vai para todas, como "Aplicar a todas".

use lamina_core::{Border, FrameContent, Mm, Opacity, PageSide, Project, Rgb};

use super::values::{color, within};
use super::{
    Agent, LookCall, PartArg, ToolError, ToolReply, edit_on, photo_frame, session, spread_index,
};
use crate::appearance::{regap, set_background, set_frame_style, style_all};
use crate::forms::LookField;
use crate::frame_form::decimal;
use crate::look_form::BORDER_FIRST;
use crate::{Editor, EditorError};

const NOTHING: &str = "Diga o que mudar: espaco_mm, fundo (e pagina), borda_mm, cor_borda, \
                       cantos_mm, opacidade (e quadro), ou todas: true para levar a aparência da lâmina a \n                       todas.";

/// As mudanças pedidas, já conferidas.
#[derive(Debug, Default)]
struct LookChange {
    gap: Option<Mm>,
    background: Option<(Rgb, Option<PageSide>)>,
    border: Option<Border>,
    radius: Option<Mm>,
    /// Spec OP: a opacidade e o quadro (sem ele, todas as fotos da lâmina).
    opacity: Option<Opacity>,
    frame: Option<usize>,
}

impl LookChange {
    fn from_call(p: &Project, index: usize, a: &LookCall) -> Result<Self, ToolError> {
        let max = |field: LookField| (0.0, field.step_and_max().1);
        let gap = a
            .espaco_mm
            .map(|v| within(v, max(LookField::Gap), "O espaço vai de 0 a 20 mm."))
            .transpose()?;
        let radius = a
            .cantos_mm
            .map(|v| within(v, max(LookField::Radius), "Os cantos vão de 0 a 10 mm."))
            .transpose()?;
        let width = a
            .borda_mm
            .map(|v| within(v, max(LookField::Border), "A borda vai de 0 a 5 mm."))
            .transpose()?;
        let border_color = a.cor_borda.as_deref().map(color).transpose()?;
        let side = match a.pagina {
            PartArg::Lamina => None,
            PartArg::Esquerda => Some(PageSide::Left),
            PartArg::Direita => Some(PageSide::Right),
        };
        let background = a.fundo.as_deref().map(color).transpose()?;
        let opacity = a
            .opacidade
            .map(|v| {
                u8::try_from(v)
                    .ok()
                    .and_then(Opacity::new)
                    .ok_or(ToolError::OutOfRange("A opacidade vai de 0 a 100."))
            })
            .transpose()?;
        Ok(LookChange {
            gap: gap.map(Mm),
            background: background.map(|c| (c, side)),
            border: border_for(p, index, width, border_color),
            radius: radius.map(Mm),
            opacity,
            frame: chosen_frame(p, index, a)?,
        })
    }

    fn is_empty(&self) -> bool {
        self.gap.is_none()
            && self.background.is_none()
            && self.border.is_none()
            && self.radius.is_none()
            && self.opacity.is_none()
    }

    /// Aplica na ordem da aba Aparência; devolve o espaço que ficou, se foi pedido.
    fn apply(&self, p: &mut Project, index: usize, all: bool) -> Result<Option<Mm>, EditorError> {
        let kept = self.gap.map(|gap| regap(p, index, gap)).transpose()?;
        if let Some((c, side)) = self.background {
            set_background(p, index, c, side)?;
        }
        if self.border.is_some() || self.radius.is_some() {
            set_frame_style(p, index, self.frame, self.border, self.radius)?;
        }
        if let Some(opacity) = self.opacity {
            set_opacity(p, index, self.frame, opacity);
        }
        if all {
            style_all(p, index)?;
        }
        Ok(kept)
    }
}

/// O quadro pedido (a partir de 1) como posição; texto, vazio ou junto com `todas` recusam.
fn chosen_frame(p: &Project, index: usize, a: &LookCall) -> Result<Option<usize>, ToolError> {
    let Some(quadro) = a.quadro else {
        return Ok(None);
    };
    if a.todas {
        return Err(ToolError::FrameAndAll);
    }
    let frame = photo_frame(p, index, quadro)?;
    if matches!(p.spreads[index].frames[frame].content, FrameContent::Empty) {
        return Err(ToolError::NoPhotoHere {
            spread: index + 1,
            frame: quadro,
        });
    }
    Ok(Some(frame))
}

/// A opacidade das fotos da lâmina, ou só do quadro `frame`.
fn set_opacity(p: &mut Project, index: usize, frame: Option<usize>, opacity: Opacity) {
    let Some(spread) = p.spreads.get_mut(index) else {
        return;
    };
    for (i, f) in spread.frames.iter_mut().enumerate() {
        if frame.is_none_or(|only| only == i) && matches!(f.content, FrameContent::Photo { .. }) {
            f.opacity = opacity;
        }
    }
}

/// A borda nova das fotos: a espessura e a cor pedidas, com o que faltar vindo da 1ª foto da
/// lâmina; só a cor, numa lâmina sem borda, ganha a espessura da janela ([`BORDER_FIRST`]).
fn border_for(p: &Project, index: usize, width: Option<f64>, c: Option<Rgb>) -> Option<Border> {
    if width.is_none() && c.is_none() {
        return None;
    }
    let now = p.spreads[index]
        .frames
        .iter()
        .find(|f| matches!(f.content, FrameContent::Photo { .. }))
        .map(|f| f.border)
        .unwrap_or_default();
    let width = width.unwrap_or(if now.width.get() > 0.0 {
        now.width.get()
    } else {
        BORDER_FIRST
    });
    Some(Border {
        width: Mm(width),
        color: c.unwrap_or(now.color),
    })
}

impl Agent {
    pub(super) fn look(
        &mut self,
        editor: &mut Editor,
        a: &LookCall,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let change = LookChange::from_call(p, index, a)?;
        if change.is_empty() && !a.todas {
            return Err(ToolError::NothingAsked(NOTHING));
        }
        let label = if a.todas {
            "Aparência · todas as lâminas"
        } else {
            "Aparência"
        };
        let mut kept = None;
        edit_on(editor, index, label, |p| {
            kept = change.apply(p, index, a.todas)?;
            Ok(())
        })?;
        let mut text = if a.todas {
            format!(
                "Aparência da lâmina {} levada a todas as lâminas.",
                a.lamina
            )
        } else {
            format!("Aparência da lâmina {} mudada.", a.lamina)
        };
        if let (Some(kept), Some(asked)) = (kept, change.gap)
            && (kept.get() - asked.get()).abs() > 1e-9
        {
            let mm = decimal(kept.get());
            text.push_str(&format!(
                " O espaço foi reduzido para {mm} mm: não cabia mais."
            ));
        }
        self.edited(editor, index, &text)
    }
}
