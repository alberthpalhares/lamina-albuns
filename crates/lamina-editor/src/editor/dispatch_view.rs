//! O registro de comandos no editor, parte de vista: navegar entre as lâminas, as faixas, o
//! painel, a faixa de layouts, as guias, o movimento e o zoom. Só mudam o que a janela mostra,
//! nunca o álbum (a janela redesenha só a lâmina atual).

use super::Editor;
use crate::{Command, Strip};

impl Editor {
    /// O comando de vista pode rodar agora? `None` = não é de vista.
    pub(super) fn view_available(&self, command: Command) -> Option<bool> {
        let s = self.session.as_ref()?;
        let last = s.spread_count().saturating_sub(1);
        let ok = match command {
            Command::NextSpread | Command::LastSpread => s.current() < last,
            Command::PreviousSpread | Command::FirstSpread => s.current() > 0,
            Command::ToggleGuides | Command::ToggleReduceMotion | Command::TogglePanel => true,
            Command::ToggleSpreads | Command::ToggleTimeline | Command::TogglePhotos => true,
            Command::AdjustPhoto => self.photo_adjust().is_some(),
            // A faixa de layouts precisa de uma lâmina (spec FL, regra 10).
            Command::ToggleLayoutBand => s.spread_count() > 0,
            // Rodada 0b: já no registro, mas inertes até a spec de cada um ligar (ZR).
            Command::Loupe | Command::ActualSize | Command::CalibrateSize => false,
            _ => return None,
        };
        Some(ok)
    }

    /// Roda os comandos de vista; `None` = não é de vista.
    pub(super) fn view_run(&mut self, command: Command) -> Option<bool> {
        let done = match command {
            Command::ToggleGuides => {
                self.guides = !self.guides;
                true
            }
            Command::ToggleSpreads => self.toggle_strip(Strip::Spreads),
            Command::ToggleTimeline => self.toggle_strip(Strip::Timeline),
            Command::TogglePhotos => self.toggle_strip(Strip::Photos),
            Command::TogglePanel => self.toggle_panel(),
            Command::AdjustPhoto => {
                self.wants_details = true;
                self.ask_panel_for_selected();
                true
            }
            Command::ToggleReduceMotion => self.toggle_reduce_motion(),
            Command::ToggleLayoutBand => {
                // Os quadros da prévia são outros: abrir desmarca o quadro (spec FL, regra 2).
                self.selected_frame = None;
                self.toggle_band()
            }
            Command::NextSpread
            | Command::PreviousSpread
            | Command::FirstSpread
            | Command::LastSpread => {
                self.selected_frame = None;
                self.navigate(command)
            }
            _ => return None,
        };
        Some(done)
    }

    fn navigate(&mut self, command: Command) -> bool {
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let target = match command {
            Command::NextSpread => s.current() + 1,
            Command::PreviousSpread => s.current().saturating_sub(1),
            Command::FirstSpread => 0,
            Command::LastSpread => s.spread_count().saturating_sub(1),
            _ => return false,
        };
        s.go_to(target)
    }
}
