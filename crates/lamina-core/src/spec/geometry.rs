//! Geometria da especificação: corte final, canvas, páginas e zona segura, em mm e em pixels.

use super::{AlbumSpec, PageSide};
use crate::{Mm, Rect};

impl AlbumSpec {
    /// Lâmina aberta no corte final (duas páginas, sem sangria), com origem no canvas.
    pub fn trim_rect(&self) -> Rect {
        Rect::new(
            self.bleed.outer,
            self.bleed.top_bottom,
            self.page_width * 2.0,
            self.page_height,
        )
    }

    /// Canvas inteiro: corte final + sangria externa e de cima/baixo. É o tamanho do arquivo
    /// exportado de uma lâmina.
    pub fn canvas_rect(&self) -> Rect {
        let trim = self.trim_rect();
        Rect::new(
            Mm::ZERO,
            Mm::ZERO,
            trim.width + self.bleed.outer * 2.0,
            trim.height + self.bleed.top_bottom * 2.0,
        )
    }

    /// Uma das páginas da lâmina, no corte final.
    pub fn page_rect(&self, side: PageSide) -> Rect {
        let trim = self.trim_rect();
        let x = match side {
            PageSide::Left => trim.x,
            PageSide::Right => trim.x + self.page_width,
        };
        Rect::new(x, trim.y, self.page_width, self.page_height)
    }

    /// Página com a própria sangria (incluindo a interna): o que vira um arquivo quando a
    /// lâmina é exportada como páginas avulsas.
    pub fn page_canvas_rect(&self, side: PageSide) -> Rect {
        self.bleed.outset_page(self.page_rect(side), side)
    }

    /// Zona segura de uma página: textos e rostos devem ficar aqui dentro.
    pub fn safe_rect(&self, side: PageSide) -> Rect {
        self.safe_margins.inset_page(self.page_rect(side), side)
    }

    /// Tamanho do arquivo exportado de uma lâmina, em pixels (largura, altura).
    pub fn canvas_size_px(&self) -> (u32, u32) {
        let canvas = self.canvas_rect();
        (canvas.width.to_px(self.dpi), canvas.height.to_px(self.dpi))
    }

    /// Tamanho da lâmina no corte final, em pixels (largura, altura).
    pub fn trim_size_px(&self) -> (u32, u32) {
        let trim = self.trim_rect();
        (trim.width.to_px(self.dpi), trim.height.to_px(self.dpi))
    }
}
