//! O registro de comandos no editor: quando cada comando está disponível e o que ele faz.

use super::{Editor, adjust, crop, organize_cmd};
use crate::suggestions::Part;
use crate::{Command, Session, Strip};

impl Editor {
    /// O comando pode rodar agora? (menu e teclado obedecem; regra 10 do JP)
    pub fn available(&self, command: Command) -> bool {
        // Durante uma tarefa longa, só o Sobre e a ajuda de sugestões e registros (spec SR,
        // regra 2): é quando algo trava que o testador quer relatar.
        if self.busy {
            return matches!(
                command,
                Command::About | Command::SendFeedback | Command::OpenLogs
            );
        }
        let Some(s) = &self.session else {
            return matches!(
                command,
                Command::NewAlbum
                    | Command::Open
                    | Command::OpenExample
                    | Command::ToggleReduceMotion
                    | Command::About
                    | Command::Presets
                    | Command::Models
                    | Command::CheckUpdates
                    | Command::ToggleAutoUpdates
                    | Command::ToggleBeta
                    | Command::SendFeedback
                    | Command::OpenLogs
            ) || (command == Command::ClearRecents && self.recents.has_unpinned());
        };
        // Atalhos por modo (spec EG, regra 2) e os comandos do modo organizar.
        if !command.in_mode(self.mode()) {
            return false;
        }
        if let Some(ok) = self
            .organize_available(command)
            .or_else(|| self.arrange_available(command))
        {
            return ok;
        }
        let last = s.spread_count().saturating_sub(1);
        match command {
            Command::NewAlbum | Command::Open | Command::Save | Command::SaveAs => true,
            Command::OpenExample | Command::CloseAlbum => true,
            Command::ClearRecents => self.recents.has_unpinned(),
            Command::Export | Command::Specification | Command::Presets => true,
            Command::MountAlbum | Command::ImportPhotos | Command::ImportFolder => true,
            Command::FindMissing => !s.missing().is_empty(),
            Command::ToggleGuides | Command::About | Command::ToggleReordered => true,
            Command::ToggleSpreads | Command::ToggleTimeline | Command::TogglePhotos => true,
            Command::ToggleReduceMotion => true,
            Command::NextSpread | Command::LastSpread => s.current() < last,
            Command::PreviousSpread | Command::FirstSpread => s.current() > 0,
            Command::Undo => s.can_undo(),
            Command::Redo => s.can_redo(),
            Command::NextLayout | Command::PreviousLayout => self.can_step(Part::Spread),
            Command::NextLeft | Command::PreviousLeft => self.can_step(Part::Left),
            Command::NextRight | Command::PreviousRight => self.can_step(Part::Right),
            Command::Mirror => self.can_mirror(),
            Command::Favorite | Command::Reject => self.panel(Part::Spread).in_use.is_some(),
            Command::ClearFrame if self.organizing() => !self.selected_frames().is_empty(),
            Command::ClearFrame => self.selected_content().is_some(),
            Command::NewText | Command::NewPhotoFrame | Command::Models => true,
            Command::CheckUpdates | Command::ToggleAutoUpdates | Command::ToggleBeta => true,
            Command::SendFeedback | Command::OpenLogs => true,
            Command::SaveModel => self.can_save_model(),
            Command::BringForward | Command::SendBackward => self.can_reorder(),
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
            // Os do modo organizar já responderam acima.
            Command::Organize
            | Command::SelectAllFrames
            | Command::NudgeLeft
            | Command::NudgeRight
            | Command::NudgeUp
            | Command::NudgeDown
            | Command::NudgeLeftMore
            | Command::NudgeRightMore
            | Command::NudgeUpMore
            | Command::NudgeDownMore
            | Command::ViewZoomIn
            | Command::ViewZoomOut
            | Command::ViewFit
            | Command::AlignLeft
            | Command::AlignCenter
            | Command::AlignRight
            | Command::AlignTop
            | Command::AlignMiddle
            | Command::AlignBottom
            | Command::EqualWidth
            | Command::EqualHeight
            | Command::EqualSize
            | Command::DistributeAcross
            | Command::DistributeDown => false,
            Command::CopyFrames | Command::DuplicateFrames => !self.selected_frames().is_empty(),
            Command::PasteFrames => self.can_paste(),
            Command::TurnLeft
            | Command::TurnRight
            | Command::TurnLeftMore
            | Command::TurnRightMore => self.crop_frame.is_some(),
        }
    }

    /// Roda os comandos que não precisam de diálogo. `true` = algo mudou. Os de arquivo pedem um
    /// caminho à janela e chamam [`Editor::open`] e a sessão.
    pub fn run(&mut self, command: Command) -> bool {
        if !self.available(command) {
            return false;
        }
        // Um comando no meio do arrasto de um quadro descarta o gesto: soltar depois gravaria o
        // quadro na lâmina errada (auditoria do Marco 3, M2). E encerra a emenda do histórico:
        // digitar depois de um comando é uma edição nova (spec TX, regra 10).
        self.gesture = None;
        self.group = None;
        // Setas seguidas no mesmo grupo viram uma edição só (spec EG, regra 10).
        let nudging =
            organize_cmd::nudge_of(command).is_some() || adjust::turn_of(command).is_some();
        if let Some(s) = self.session.as_mut()
            && !nudging
        {
            s.end_merge();
        }
        let had_crop = self.crop_frame.is_some();
        // Qualquer outro comando fecha o modo recorte, gravando o gesto (spec RC, regra 1).
        let crop_command = matches!(
            command,
            Command::CropMode | Command::ZoomIn | Command::ZoomOut | Command::ResetCrop
        ) || adjust::turn_of(command).is_some();
        if !crop_command {
            self.exit_crop();
        }
        if let Some(done) = self.organize_run(command, had_crop) {
            return done;
        }
        if let Some(done) = self.arrange_run(command) {
            return done;
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
            Command::ToggleSpreads => self.toggle_strip(Strip::Spreads),
            Command::ToggleTimeline => self.toggle_strip(Strip::Timeline),
            Command::TogglePhotos => self.toggle_strip(Strip::Photos),
            Command::ToggleReduceMotion => self.toggle_reduce_motion(),
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
            Command::ClearFrame if self.organizing() => self.delete_group(),
            Command::ClearFrame => self.delete_selected(),
            Command::CopyFrames => self.copy_frames(),
            Command::PasteFrames => self.paste_frames(),
            Command::DuplicateFrames => self.duplicate_frames(),
            c if adjust::turn_of(c).is_some() => {
                adjust::turn_of(c).is_some_and(|t| self.adjust(adjust::AdjustChange::Turn(t)))
            }
            Command::NewText => self.new_text(None),
            Command::NewPhotoFrame => self.new_photo_frame(None),
            Command::BringForward => self.reorder_selected(true),
            Command::SendBackward => self.reorder_selected(false),
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
        self.gesture = None;
        self.selected_frame = None;
        self.session.as_mut().is_some_and(|s| s.go_to_step(step))
    }
}
