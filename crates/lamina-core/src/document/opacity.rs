//! A opacidade da foto num quadro (spec OP): de 0 a 100%, por quadro, como a borda e os cantos.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Opacidade em % (0 a 100; 100 = a foto opaca, como sempre). O arquivo aceita qualquer inteiro
/// de 0 a 255 e [`crate::Project::validate`] recusa o que passa de 100 (regra 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Opacity(u8);

impl Opacity {
    /// A foto opaca: o padrão, que não vai para o arquivo.
    pub const FULL: Opacity = Opacity(100);

    /// `None` acima de 100.
    pub fn new(percent: u8) -> Option<Opacity> {
        (percent <= 100).then_some(Opacity(percent))
    }

    /// O valor em % (de um arquivo ainda não validado, pode passar de 100).
    pub fn get(self) -> u8 {
        self.0
    }

    /// De 0 a 1, para misturar a foto com o que está embaixo (regra 3).
    pub fn fraction(self) -> f32 {
        f32::from(self.0.min(100)) / 100.0
    }

    pub fn is_full(&self) -> bool {
        self.0 == 100
    }

    pub fn is_valid(self) -> bool {
        self.0 <= 100
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Opacity::FULL
    }
}
