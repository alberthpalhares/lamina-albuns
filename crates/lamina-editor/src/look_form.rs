//! Os botões − e + e a cor da borda do painel de aparência (spec AP, seção 3). Saiu do
//! `forms.rs` (auditoria do Marco 3, M10).

use lamina_core::{Mm, Rgb};

use crate::Editor;
use crate::editor::Look;

/// O que os botões − e + do painel de aparência mudam (spec AP, seção 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookField {
    Gap,
    Border,
    Radius,
}

impl LookField {
    /// Passo de cada clique e o máximo, em mm.
    pub fn step_and_max(self) -> (f64, f64) {
        match self {
            LookField::Gap => (0.5, 20.0),
            LookField::Border => (0.5, 5.0),
            LookField::Radius => (1.0, 10.0),
        }
    }

    fn value(self, look: &Look) -> f64 {
        match self {
            LookField::Gap => look.gap.get(),
            LookField::Border => look.border.width.get(),
            LookField::Radius => look.radius.get(),
        }
    }
}

/// Borda que uma cor escolhida sem espessura ganha, em mm.
pub const BORDER_FIRST: f64 = 1.0;

impl Editor {
    /// Um clique em − (`delta` < 0) ou + do painel de aparência. Devolve o aviso do editor (o
    /// espaço reduzido para caber), se houver.
    pub fn step_look(&mut self, field: LookField, delta: i32) -> Option<String> {
        let look = self.look()?;
        let (step, max) = field.step_and_max();
        let value = Mm((field.value(&look) + f64::from(delta) * step).clamp(0.0, max));
        match field {
            LookField::Gap => self.set_gap(value),
            LookField::Border => {
                self.set_border(value, look.border.color);
                None
            }
            LookField::Radius => {
                self.set_radius(value);
                None
            }
        }
    }

    /// Escolher a cor da borda: sem espessura ainda, a borda ganha [`BORDER_FIRST`].
    pub fn pick_border_color(&mut self, color: Rgb) -> bool {
        let Some(look) = self.look() else {
            return false;
        };
        let width = if look.border.width.get() > 0.0 {
            look.border.width
        } else {
            Mm(BORDER_FIRST)
        };
        self.set_border(width, color)
    }
}
