//! Margens de página medidas a partir das bordas, com a lombada tratada à parte.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PageSide;
use crate::{CoreError, Mm, Rect};

/// Margens de uma página, a partir de cada borda.
///
/// Numa lâmina aberta, `outer` é a borda externa (a esquerda da página esquerda e a direita da
/// página direita) e `inner` é a borda da lombada. Laboratórios costumam pedir valores
/// diferentes para cada uma (estudo E2).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Margins {
    pub outer: Mm,
    pub top_bottom: Mm,
    pub inner: Mm,
}

impl Margins {
    pub const ZERO: Margins = Margins {
        outer: Mm::ZERO,
        top_bottom: Mm::ZERO,
        inner: Mm::ZERO,
    };

    pub fn new(outer: Mm, top_bottom: Mm, inner: Mm) -> Self {
        Margins {
            outer,
            top_bottom,
            inner,
        }
    }

    /// O mesmo valor nas três bordas.
    pub fn uniform(value: Mm) -> Self {
        Margins::new(value, value, value)
    }

    /// Confere que todas as margens são finitas e não negativas. `name` identifica a margem no
    /// erro ("bleed" ou "safe").
    pub fn validate(&self, name: &'static str) -> Result<(), CoreError> {
        let sides = [
            ("outer", self.outer),
            ("top_bottom", self.top_bottom),
            ("inner", self.inner),
        ];
        for (side, value) in sides {
            if !value.is_finite_non_negative() {
                return Err(CoreError::InvalidMargin {
                    margin: name,
                    side,
                    value: value.get(),
                });
            }
        }
        Ok(())
    }

    /// Retângulo de `page` encolhido pelas margens. A lombada fica à direita da página
    /// esquerda e à esquerda da página direita.
    pub fn inset_page(&self, page: Rect, side: PageSide) -> Rect {
        let left = match side {
            PageSide::Left => self.outer,
            PageSide::Right => self.inner,
        };
        Rect::new(
            page.x + left,
            page.y + self.top_bottom,
            page.width - self.outer - self.inner,
            page.height - self.top_bottom * 2.0,
        )
    }

    /// Retângulo de `page` aumentado pelas margens (usado para a sangria de página avulsa).
    pub fn outset_page(&self, page: Rect, side: PageSide) -> Rect {
        let left = match side {
            PageSide::Left => self.outer,
            PageSide::Right => self.inner,
        };
        Rect::new(
            page.x - left,
            page.y - self.top_bottom,
            page.width + self.outer + self.inner,
            page.height + self.top_bottom * 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(x: f64) -> Rect {
        Rect::new(Mm(x), Mm(0.0), Mm(300.0), Mm(300.0))
    }

    #[test]
    fn lombada_fica_do_lado_de_dentro_de_cada_pagina() {
        let m = Margins::new(Mm(10.0), Mm(12.0), Mm(25.0));
        let left = m.inset_page(page(0.0), PageSide::Left);
        let right = m.inset_page(page(300.0), PageSide::Right);
        assert_eq!(left, Rect::new(Mm(10.0), Mm(12.0), Mm(265.0), Mm(276.0)));
        assert_eq!(right, Rect::new(Mm(325.0), Mm(12.0), Mm(265.0), Mm(276.0)));
    }

    #[test]
    fn outset_e_o_inverso_de_inset() {
        let m = Margins::new(Mm(3.0), Mm(3.0), Mm(5.0));
        for side in [PageSide::Left, PageSide::Right] {
            let p = page(300.0);
            assert_eq!(m.outset_page(m.inset_page(p, side), side), p);
        }
    }

    #[test]
    fn margem_negativa_e_rejeitada_com_o_lado_certo() {
        let m = Margins::new(Mm(1.0), Mm(1.0), Mm(-2.0));
        let err = m.validate("safe").unwrap_err();
        assert!(matches!(
            err,
            CoreError::InvalidMargin {
                margin: "safe",
                side: "inner",
                ..
            }
        ));
    }
}
