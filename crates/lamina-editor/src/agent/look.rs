//! Aparência pela IA (spec IA, `aparencia`; spec AP): espaço entre fotos, fundo, borda e cantos
//! numa edição só, e com `todas` a aparência da lâmina vai para todas, como "Aplicar a todas".

use lamina_core::{Border, FrameContent, Mm, PageSide, Project, Rgb};

use super::values::{color, within};
use super::{Agent, LookCall, PartArg, ToolError, ToolReply, edit_on, session, spread_index};
use crate::appearance::{regap, set_background, set_frame_style, style_all};
use crate::forms::LookField;
use crate::frame_form::decimal;
use crate::look_form::BORDER_FIRST;
use crate::{Editor, EditorError};

const NOTHING: &str = "Diga o que mudar: espaco_mm, fundo (e pagina), borda_mm, cor_borda, \
                       cantos_mm, ou todas: true para levar a aparência da lâmina a todas.";

/// As mudanças pedidas, já conferidas.
#[derive(Debug, Default)]
struct LookChange {
    gap: Option<Mm>,
    background: Option<(Rgb, Option<PageSide>)>,
    border: Option<Border>,
    radius: Option<Mm>,
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
        Ok(LookChange {
            gap: gap.map(Mm),
            background: background.map(|c| (c, side)),
            border: border_for(p, index, width, border_color),
            radius: radius.map(Mm),
        })
    }

    fn is_empty(&self) -> bool {
        self.gap.is_none()
            && self.background.is_none()
            && self.border.is_none()
            && self.radius.is_none()
    }

    /// Aplica na ordem da aba Aparência; devolve o espaço que ficou, se foi pedido.
    fn apply(&self, p: &mut Project, index: usize, all: bool) -> Result<Option<Mm>, EditorError> {
        let kept = self.gap.map(|gap| regap(p, index, gap)).transpose()?;
        if let Some((c, side)) = self.background {
            set_background(p, index, c, side)?;
        }
        if self.border.is_some() || self.radius.is_some() {
            set_frame_style(p, index, None, self.border, self.radius)?;
        }
        if all {
            style_all(p, index)?;
        }
        Ok(kept)
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
