//! Geometria do documento.
//!
//! - [`Rect`]: retângulo absoluto em milímetros. Origem no canto superior esquerdo, y para baixo.
//! - [`RelRect`]: retângulo relativo, em frações (0..=1) de uma área de referência.
//!
//! Os quadros guardam [`RelRect`]: assim um layout reflui sozinho quando o tamanho do
//! álbum muda (ADR 0003).

use serde::{Deserialize, Serialize};

use crate::{CoreError, Mm};

/// Folga para erros de arredondamento de ponto flutuante.
const EPSILON: f64 = 1e-9;

/// Retângulo absoluto, em milímetros.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
}

impl Rect {
    pub fn new(x: Mm, y: Mm, width: Mm, height: Mm) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> Mm {
        self.x + self.width
    }

    pub fn bottom(&self) -> Mm {
        self.y + self.height
    }

    /// Encolhe o retângulo `amount` para dentro em todos os lados.
    pub fn inset(&self, amount: Mm) -> Rect {
        Rect::new(
            self.x + amount,
            self.y + amount,
            self.width - amount * 2.0,
            self.height - amount * 2.0,
        )
    }

    /// Aumenta o retângulo `amount` para fora em todos os lados.
    pub fn outset(&self, amount: Mm) -> Rect {
        self.inset(Mm(-amount.get()))
    }

    /// `true` se `other` está inteiramente dentro deste retângulo.
    pub fn contains_rect(&self, other: &Rect) -> bool {
        other.x.get() >= self.x.get() - EPSILON
            && other.y.get() >= self.y.get() - EPSILON
            && other.right().get() <= self.right().get() + EPSILON
            && other.bottom().get() <= self.bottom().get() + EPSILON
    }

    /// Largura dividida pela altura.
    pub fn aspect_ratio(&self) -> f64 {
        self.width.get() / self.height.get()
    }
}

/// Retângulo relativo: posição e tamanho como frações de uma área de referência.
///
/// Invariantes (garantidas por [`RelRect::new`] e verificadas por [`RelRect::validate`]
/// ao carregar um projeto): valores finitos, `x, y >= 0`, `width, height > 0`,
/// e o retângulo cabe em `0..=1` nos dois eixos.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RelRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl RelRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Result<Self, CoreError> {
        let rect = RelRect {
            x,
            y,
            width,
            height,
        };
        rect.validate()?;
        Ok(rect)
    }

    /// A área de referência inteira.
    pub fn full() -> Self {
        RelRect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        let values = [self.x, self.y, self.width, self.height];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(CoreError::InvalidRelRect(format!(
                "valor não finito em {self:?}"
            )));
        }
        if self.x < 0.0 || self.y < 0.0 {
            return Err(CoreError::InvalidRelRect(format!(
                "posição negativa em {self:?}"
            )));
        }
        if self.width <= 0.0 || self.height <= 0.0 {
            return Err(CoreError::InvalidRelRect(format!(
                "tamanho zero ou negativo em {self:?}"
            )));
        }
        if self.x + self.width > 1.0 + EPSILON || self.y + self.height > 1.0 + EPSILON {
            return Err(CoreError::InvalidRelRect(format!(
                "sai da área de referência: {self:?}"
            )));
        }
        Ok(())
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    /// Converte para milímetros dentro de `area`.
    pub fn to_absolute(&self, area: &Rect) -> Rect {
        Rect::new(
            area.x + area.width * self.x,
            area.y + area.height * self.y,
            area.width * self.width,
            area.height * self.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::new(Mm(x), Mm(y), Mm(w), Mm(h))
    }

    #[test]
    fn metade_esquerda_de_uma_lamina() {
        let area = rect(3.0, 3.0, 600.0, 300.0);
        let left = RelRect::new(0.0, 0.0, 0.5, 1.0).unwrap();
        assert_eq!(left.to_absolute(&area), rect(3.0, 3.0, 300.0, 300.0));
    }

    #[test]
    fn mesmo_layout_reflui_em_outro_tamanho() {
        let frame = RelRect::new(0.25, 0.1, 0.5, 0.8).unwrap();
        let square = frame.to_absolute(&rect(0.0, 0.0, 400.0, 200.0));
        let wide = frame.to_absolute(&rect(0.0, 0.0, 600.0, 200.0));
        assert_eq!(square, rect(100.0, 20.0, 200.0, 160.0));
        assert_eq!(wide, rect(150.0, 20.0, 300.0, 160.0));
    }

    #[test]
    fn rejeita_retangulos_invalidos() {
        let invalid =
            |r: Result<RelRect, CoreError>| matches!(r, Err(CoreError::InvalidRelRect(_)));
        assert!(invalid(RelRect::new(-0.1, 0.0, 0.5, 0.5)));
        assert!(invalid(RelRect::new(0.0, 0.0, 0.0, 0.5)));
        assert!(invalid(RelRect::new(0.6, 0.0, 0.5, 0.5)));
        assert!(invalid(RelRect::new(f64::NAN, 0.0, 0.5, 0.5)));
    }

    #[test]
    fn aceita_soma_com_erro_de_arredondamento() {
        // 0,1 + 0,2 = 0,30000000000000004 em ponto flutuante.
        RelRect::new(0.7, 0.0, 0.1 + 0.2, 1.0).unwrap();
    }

    #[test]
    fn inset_e_outset_sao_inversos() {
        let r = rect(10.0, 10.0, 100.0, 50.0);
        assert_eq!(r.inset(Mm(5.0)), rect(15.0, 15.0, 90.0, 40.0));
        assert_eq!(r.inset(Mm(5.0)).outset(Mm(5.0)), r);
    }

    #[test]
    fn contains_rect() {
        let outer = rect(0.0, 0.0, 100.0, 100.0);
        assert!(outer.contains_rect(&rect(10.0, 10.0, 50.0, 50.0)));
        assert!(!outer.contains_rect(&rect(60.0, 10.0, 50.0, 50.0)));
    }
}
