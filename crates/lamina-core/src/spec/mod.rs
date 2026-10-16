//! Especificação física do álbum.
//!
//! Sistema de coordenadas da **tela da lâmina** (canvas), em mm:
//!
//! ```text
//! (0,0) ┌──────────────────────────────────────┐  ← borda da sangria (bleed)
//!       │  ┌────────────────┬───────────────┐  │
//!       │  │ página esquerda│ página direita│  │  ← corte final (trim)
//!       │  │   ┌─────────┐  │  ┌─────────┐  │  │
//!       │  │   │ segura  │  │  │ segura  │  │  │  ← zona segura (só visual)
//!       │  │   └─────────┘  │  └─────────┘  │  │
//!       │  └────────────────┴───────────────┘  │
//!       └──────────────────────────────────────┘
//!                           ↑ lombada (margem interna)
//! ```
//!
//! A lâmina aberta tem duas páginas lado a lado. O arquivo exportado inclui a sangria externa e
//! a de cima/baixo; a sangria interna só entra quando a lâmina é exportada como páginas avulsas.

mod edges;
mod geometry;
mod margins;

pub use edges::{ActivePages, AlbumEdge, Edges};
pub use margins::Margins;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{CoreError, Dpi, Mm};

/// Como o usuário diagrama o álbum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DesignMode {
    /// Lâmina inteira: quadros podem atravessar o centro (encadernação lay-flat).
    #[default]
    Spread,
    /// Página a página: nada atravessa o centro (laboratórios que imprimem páginas soltas).
    Page,
}

/// Lado de uma página dentro da lâmina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PageSide {
    Left,
    Right,
}

/// Especificação física do álbum: tamanho da página, resolução, sangria, zona segura e como o
/// miolo começa e termina.
///
/// Construa com [`AlbumSpec::new`] e ajuste com os métodos `with_*`. Os campos são privados
/// para que uma especificação inválida não possa existir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AlbumSpec {
    page_width: Mm,
    page_height: Mm,
    dpi: Dpi,
    bleed: Margins,
    safe_margins: Margins,
    mode: DesignMode,
    edges: Edges,
}

impl AlbumSpec {
    /// Nova especificação sem sangria, sem zona segura e com lâminas inteiras nas pontas.
    pub fn new(page_width: Mm, page_height: Mm, dpi: Dpi) -> Result<Self, CoreError> {
        let spec = AlbumSpec {
            page_width,
            page_height,
            dpi,
            bleed: Margins::ZERO,
            safe_margins: Margins::ZERO,
            mode: DesignMode::default(),
            edges: Edges::default(),
        };
        spec.validate()?;
        Ok(spec)
    }

    /// Mesma sangria nas três bordas.
    pub fn with_bleed(self, bleed: Mm) -> Result<Self, CoreError> {
        self.with_bleed_margins(Margins::uniform(bleed))
    }

    pub fn with_bleed_margins(mut self, bleed: Margins) -> Result<Self, CoreError> {
        self.bleed = bleed;
        self.validate()?;
        Ok(self)
    }

    /// Mesma zona segura nas três bordas.
    pub fn with_safe_margin(self, margin: Mm) -> Result<Self, CoreError> {
        self.with_safe_margins(Margins::uniform(margin))
    }

    pub fn with_safe_margins(mut self, margins: Margins) -> Result<Self, CoreError> {
        self.safe_margins = margins;
        self.validate()?;
        Ok(self)
    }

    pub fn with_mode(mut self, mode: DesignMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_edges(mut self, start: AlbumEdge, end: AlbumEdge) -> Self {
        self.edges = Edges::new(start, end);
        self
    }

    /// Confere as invariantes. Chamado pelos construtores e ao carregar um projeto.
    pub fn validate(&self) -> Result<(), CoreError> {
        let sizes = [
            ("page_width", self.page_width),
            ("page_height", self.page_height),
        ];
        for (field, value) in sizes {
            if !value.is_finite_positive() {
                return Err(CoreError::InvalidLength {
                    field,
                    value: value.get(),
                });
            }
        }
        self.bleed.validate("bleed")?;
        self.safe_margins.validate("safe")?;
        let m = self.safe_margins;
        let free_width = self.page_width - m.outer - m.inner;
        let free_height = self.page_height - m.top_bottom * 2.0;
        if free_width.get() <= 0.0 || free_height.get() <= 0.0 {
            return Err(CoreError::SafeMarginTooLarge {
                free_width: free_width.get(),
                free_height: free_height.get(),
            });
        }
        Ok(())
    }

    pub fn page_width(&self) -> Mm {
        self.page_width
    }

    pub fn page_height(&self) -> Mm {
        self.page_height
    }

    pub fn dpi(&self) -> Dpi {
        self.dpi
    }

    pub fn bleed(&self) -> Margins {
        self.bleed
    }

    pub fn safe_margins(&self) -> Margins {
        self.safe_margins
    }

    pub fn mode(&self) -> DesignMode {
        self.mode
    }

    pub fn edges(&self) -> Edges {
        self.edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rect;

    fn dpi() -> Dpi {
        Dpi::new(300).unwrap()
    }

    /// Álbum 30×30 cm, 300 DPI, 3 mm de sangria, 10 mm de zona segura.
    fn spec_30x30() -> AlbumSpec {
        AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), dpi())
            .unwrap()
            .with_bleed(Mm(3.0))
            .unwrap()
            .with_safe_margin(Mm(10.0))
            .unwrap()
    }

    #[test]
    fn lamina_30x30_a_300_dpi_em_pixels() {
        let spec = spec_30x30();
        // 600 × 300 mm → 7086,6 × 3543,3 px
        assert_eq!(spec.trim_size_px(), (7087, 3543));
        // 606 × 306 mm → 7157,5 × 3614,2 px
        assert_eq!(spec.canvas_size_px(), (7157, 3614));
    }

    #[test]
    fn trim_fica_deslocado_pela_sangria() {
        let spec = spec_30x30();
        assert_eq!(
            spec.trim_rect(),
            Rect::new(Mm(3.0), Mm(3.0), Mm(600.0), Mm(300.0))
        );
        assert_eq!(
            spec.canvas_rect(),
            Rect::new(Mm(0.0), Mm(0.0), Mm(606.0), Mm(306.0))
        );
    }

    #[test]
    fn paginas_e_zonas_seguras() {
        let spec = spec_30x30();
        let right = spec.page_rect(PageSide::Right);
        assert_eq!(right, Rect::new(Mm(303.0), Mm(3.0), Mm(300.0), Mm(300.0)));
        let safe = spec.safe_rect(PageSide::Right);
        assert_eq!(safe, Rect::new(Mm(313.0), Mm(13.0), Mm(280.0), Mm(280.0)));
        assert!(spec.trim_rect().contains_rect(&safe));
    }

    #[test]
    fn margem_interna_maior_protege_a_lombada() {
        let spec = spec_30x30()
            .with_safe_margins(Margins::new(Mm(10.0), Mm(10.0), Mm(25.0)))
            .unwrap();
        let left = spec.safe_rect(PageSide::Left);
        let right = spec.safe_rect(PageSide::Right);
        // Página esquerda: lombada à direita (termina em 303 − 25 = 278).
        assert_eq!(left.right(), Mm(278.0));
        // Página direita: lombada à esquerda (começa em 303 + 25 = 328).
        assert_eq!(right.x, Mm(328.0));
    }

    #[test]
    fn sangria_diferente_por_borda() {
        let spec = spec_30x30()
            .with_bleed_margins(Margins::new(Mm(5.0), Mm(3.0), Mm(2.0)))
            .unwrap();
        assert_eq!(
            spec.canvas_rect(),
            Rect::new(Mm(0.0), Mm(0.0), Mm(610.0), Mm(306.0))
        );
        // Página avulsa: 5 mm por fora + 300 + 2 mm na lombada.
        let page = spec.page_canvas_rect(PageSide::Left);
        assert_eq!(page, Rect::new(Mm(0.0), Mm(0.0), Mm(307.0), Mm(306.0)));
    }

    #[test]
    fn rejeita_especificacoes_invalidas() {
        assert!(matches!(
            AlbumSpec::new(Mm(0.0), Mm(300.0), dpi()),
            Err(CoreError::InvalidLength {
                field: "page_width",
                ..
            })
        ));
        assert!(matches!(
            AlbumSpec::new(Mm(300.0), Mm(f64::INFINITY), dpi()),
            Err(CoreError::InvalidLength {
                field: "page_height",
                ..
            })
        ));
        let ok = AlbumSpec::new(Mm(300.0), Mm(200.0), dpi()).unwrap();
        assert!(matches!(
            ok.clone().with_bleed(Mm(-1.0)),
            Err(CoreError::InvalidMargin {
                margin: "bleed",
                side: "outer",
                ..
            })
        ));
        // 100 mm em cima e embaixo não cabem em 200 mm de altura.
        assert!(matches!(
            ok.clone().with_safe_margin(Mm(100.0)),
            Err(CoreError::SafeMarginTooLarge { .. })
        ));
        // 150 mm por fora + 150 mm na lombada não cabem em 300 mm de largura.
        let wide = Margins::new(Mm(150.0), Mm(0.0), Mm(150.0));
        assert!(matches!(
            ok.with_safe_margins(wide),
            Err(CoreError::SafeMarginTooLarge { .. })
        ));
    }
}
