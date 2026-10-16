//! Aparência pelo editor (spec AP): cada mudança é uma edição com rótulo no histórico.

use lamina_core::{Border, Mm, PageSide, Rgb};

use super::Editor;
use crate::appearance::{regap, set_background, set_frame_style, style_all};

/// O que o painel de aparência mostra agora.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub gap: Mm,
    /// Fundo por página ligado.
    pub split: bool,
    pub left: Rgb,
    pub right: Rgb,
    /// Borda e cantos do quadro selecionado, ou da 1ª foto da lâmina.
    pub border: Border,
    pub radius: Mm,
    /// Borda e cantos valem só para o quadro selecionado.
    pub only_frame: bool,
}

impl Editor {
    /// A aparência da lâmina atual (painel de aparência).
    pub fn look(&self) -> Option<Look> {
        let s = self.session.as_ref()?;
        let p = s.project();
        let spread = p.spreads.get(s.current())?;
        let photo =
            |f: &&lamina_core::Frame| matches!(f.content, lamina_core::FrameContent::Photo { .. });
        let frame = self
            .selected_frame
            .and_then(|i| spread.frames.get(i))
            .or_else(|| spread.frames.iter().find(photo));
        Some(Look {
            gap: p.spread_gap(spread),
            split: spread.background_right.is_some(),
            left: spread.page_background(PageSide::Left),
            right: spread.page_background(PageSide::Right),
            border: frame.map(|f| f.border).unwrap_or_default(),
            radius: frame.map_or(Mm(0.0), |f| f.radius),
            only_frame: self.selected_frame.is_some(),
        })
    }

    /// Muda o espaço entre fotos da lâmina atual. Devolve o aviso quando o espaço foi reduzido
    /// para caber (regra 1).
    pub fn set_gap(&mut self, gap: Mm) -> Option<String> {
        self.exit_crop();
        let session = self.session.as_mut()?;
        let index = session.current();
        let mut kept = gap;
        let done = session.edit("Mudar espaço", |p| {
            kept = regap(p, index, gap)?;
            Ok::<(), crate::EditorError>(())
        });
        if let Err(e) = done {
            return Some(e.to_string());
        }
        ((kept.get() - gap.get()).abs() > 1e-9).then(|| {
            let mm = format!("{}", (kept.get() * 10.0).round() / 10.0).replace('.', ",");
            format!("O espaço foi reduzido para {mm} mm: não cabia mais.")
        })
    }

    /// Fundo da lâmina atual inteira (`None`) ou de uma página.
    pub fn set_background(&mut self, color: Rgb, side: Option<PageSide>) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        session
            .edit("Mudar fundo", |p| set_background(p, index, color, side))
            .is_ok()
    }

    /// Borda do quadro selecionado ou de todas as fotos da lâmina atual.
    pub fn set_border(&mut self, width: Mm, color: Rgb) -> bool {
        let frame = self.selected_frame;
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        let border = Border { width, color };
        session
            .edit("Mudar borda", |p| {
                set_frame_style(p, index, frame, Some(border), None)
            })
            .is_ok()
    }

    /// Raio dos cantos do quadro selecionado ou de todas as fotos da lâmina atual.
    pub fn set_radius(&mut self, radius: Mm) -> bool {
        let frame = self.selected_frame;
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        session
            .edit("Mudar cantos", |p| {
                set_frame_style(p, index, frame, None, Some(radius))
            })
            .is_ok()
    }

    /// "Aplicar a todas as lâminas": uma edição só (regra 6).
    pub fn style_all(&mut self) -> bool {
        self.exit_crop();
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        session
            .edit("Aplicar aparência a todas", |p| style_all(p, index))
            .is_ok()
    }
}
