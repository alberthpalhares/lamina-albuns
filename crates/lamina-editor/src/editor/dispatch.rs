//! O registro de comandos no editor: quando cada comando está disponível e o que ele faz.

use super::{Editor, crop};
use crate::suggestions::Part;
use crate::{Command, Session};

impl Editor {
    /// O comando pode rodar agora? (menu e teclado obedecem; regra 10 do JP)
    pub fn available(&self, command: Command) -> bool {
        let Some(s) = &self.session else {
            return matches!(command, Command::NewAlbum | Command::Open | Command::About);
        };
        let last = s.spread_count().saturating_sub(1);
        match command {
            Command::NewAlbum | Command::Open | Command::Save | Command::SaveAs => true,
            Command::ToggleGuides | Command::About | Command::ToggleReordered => true,
            Command::NextSpread | Command::LastSpread => s.current() < last,
            Command::PreviousSpread | Command::FirstSpread => s.current() > 0,
            Command::Undo => s.can_undo(),
            Command::Redo => s.can_redo(),
            Command::NextLayout | Command::PreviousLayout => self.can_step(Part::Spread),
            Command::NextLeft | Command::PreviousLeft => self.can_step(Part::Left),
            Command::NextRight | Command::PreviousRight => self.can_step(Part::Right),
            Command::Mirror => self.can_mirror(),
            Command::Favorite | Command::Reject => self.panel(Part::Spread).in_use.is_some(),
            Command::ClearFrame => self.can_clear(),
            Command::Deselect => {
                self.selected_frame.is_some()
                    || !self.selection.is_empty()
                    || self.crop_frame.is_some()
            }
            Command::BuildFromSelection | Command::NewSpreadFromSelection => {
                !self.selection.is_empty() && self.photo_selection().is_ok()
            }
            Command::NewSpread => true,
            Command::DeleteSpread => s.spread_count() > 1,
            Command::MoveSpreadLeft => s.current() > 0,
            Command::MoveSpreadRight => s.current() < last,
            Command::CropMode => self.can_clear(),
            Command::ZoomIn | Command::ZoomOut | Command::ResetCrop => self.crop_frame.is_some(),
        }
    }

    /// Roda os comandos que não precisam de diálogo. `true` = algo mudou. Os de arquivo pedem um
    /// caminho à janela e chamam [`Editor::open`] e a sessão.
    pub fn run(&mut self, command: Command) -> bool {
        if !self.available(command) {
            return false;
        }
        // Qualquer outro comando fecha o modo recorte, gravando o gesto (spec RC, regra 1).
        let crop_command = matches!(
            command,
            Command::CropMode | Command::ZoomIn | Command::ZoomOut | Command::ResetCrop
        );
        if !crop_command {
            self.exit_crop();
        }
        match command {
            Command::CropMode => self.selected_frame.is_some_and(|f| self.enter_crop(f)),
            Command::ZoomIn => self.crop_zoom(crop::ZOOM_STEP) && self.commit_crop(),
            Command::ZoomOut => self.crop_zoom(1.0 / crop::ZOOM_STEP) && self.commit_crop(),
            Command::ResetCrop => self.crop_reset() && self.commit_crop(),
            Command::ToggleGuides => {
                self.guides = !self.guides;
                true
            }
            Command::ToggleReordered => {
                self.reordered = !self.reordered;
                true
            }
            Command::Undo => self.session.as_mut().and_then(Session::undo).is_some(),
            Command::Redo => self.session.as_mut().and_then(Session::redo).is_some(),
            Command::NextLayout => self.step(Part::Spread, true),
            Command::PreviousLayout => self.step(Part::Spread, false),
            Command::NextLeft => self.step(Part::Left, true),
            Command::PreviousLeft => self.step(Part::Left, false),
            Command::NextRight => self.step(Part::Right, true),
            Command::PreviousRight => self.step(Part::Right, false),
            Command::Mirror => self.mirror(),
            Command::Favorite => self.favorite(),
            Command::Reject => self.reject(),
            Command::ClearFrame => self.clear_selected(),
            Command::Deselect => {
                self.selected_frame = None;
                self.selection.clear();
                true
            }
            Command::BuildFromSelection => self.build_from_selection(0),
            Command::NewSpreadFromSelection => self.new_spread_from_selection(),
            Command::NewSpread => self.new_spread(),
            Command::DeleteSpread => self.delete_current(),
            Command::MoveSpreadLeft => self.move_by(-1),
            Command::MoveSpreadRight => self.move_by(1),
            _ => {
                self.selected_frame = None;
                self.navigate(command)
            }
        }
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

    /// Clique num passo do painel de histórico (spec HI, regra 2). Um gesto de recorte em
    /// andamento é gravado antes.
    pub fn go_to_step(&mut self, step: usize) -> bool {
        self.exit_crop();
        self.selected_frame = None;
        self.session.as_mut().is_some_and(|s| s.go_to_step(step))
    }
}
