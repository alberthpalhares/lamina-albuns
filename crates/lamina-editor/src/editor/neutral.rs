//! O clique duplo no seletor de um controle da seção Foto (spec AF, regra 14): o controle volta ao
//! neutro numa edição própria, que nunca emenda com o arrasto anterior.

use super::Editor;
use super::adjust::AdjustChange;

/// Um controle deslizante da seção Foto que volta ao neutro com clique duplo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustControl {
    Angle,
    Brightness,
    Contrast,
}

impl AdjustControl {
    /// A mudança que leva o controle ao neutro: 0°, 0 e 0.
    pub fn neutral(self) -> AdjustChange {
        match self {
            AdjustControl::Angle => AdjustChange::Angle(0),
            AdjustControl::Brightness => AdjustChange::Brightness(0),
            AdjustControl::Contrast => AdjustChange::Contrast(0),
        }
    }
}

impl Editor {
    /// Volta o controle ao neutro nas fotos selecionadas. Fecha o arrasto anterior antes e depois,
    /// para virar uma edição só dela; se já estava no neutro, nada muda.
    pub fn reset_adjust(&mut self, control: AdjustControl) -> bool {
        self.end_adjust();
        let done = self.adjust(control.neutral());
        self.end_adjust();
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutro_de_cada_controle_e_zero() {
        assert_eq!(AdjustControl::Angle.neutral(), AdjustChange::Angle(0));
        assert_eq!(
            AdjustControl::Brightness.neutral(),
            AdjustChange::Brightness(0)
        );
        assert_eq!(AdjustControl::Contrast.neutral(), AdjustChange::Contrast(0));
    }
}
