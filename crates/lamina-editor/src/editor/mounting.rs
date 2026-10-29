//! "Montagem do álbum…" (spec NP, regra 11): a etapa 2 para o álbum aberto. Vale para as próximas
//! sugestões, sem refazer as lâminas, e entra no histórico.

use lamina_core::{DesignMode, Mounting};

use super::Editor;
use crate::EditorError;
use crate::new_project::{Format, Unit};

impl Editor {
    /// A montagem e o lay-flat do álbum aberto, numa edição que Ctrl+Z desfaz.
    pub fn set_mounting(&mut self, mounting: Mounting, lay_flat: bool) -> Result<(), EditorError> {
        let mode = if lay_flat {
            DesignMode::Spread
        } else {
            DesignMode::Page
        };
        self.exit_crop();
        let session = self.session.as_mut().ok_or(EditorError::NoProject)?;
        session.edit("Montagem do álbum", |p| {
            p.mounting = mounting;
            p.spec = p.spec.clone().with_mode(mode);
            Ok::<(), EditorError>(())
        })
    }

    /// A unidade da tela, lembrada com a janela (regra 3).
    pub fn remember_unit(&mut self, unit: Unit) {
        self.strips.strips.unit = unit;
        self.save_strips();
    }

    /// O formato do último álbum criado, o cartão marcado na próxima vez (regra 1).
    pub fn remember_format(&mut self, format: Format) {
        self.strips.strips.format = format;
        self.save_strips();
    }

    /// O tamanho do último álbum criado (nome do preset e laboratório), marcado na próxima vez
    /// (spec ET, regra 1); nenhum = um tamanho personalizado.
    pub fn remember_size(&mut self, size: Option<(String, Option<String>)>) {
        self.strips.strips.last_size = size;
        self.save_strips();
    }
}
