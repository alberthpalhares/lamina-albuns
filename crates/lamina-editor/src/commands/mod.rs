//! Registro de comandos (spec JP, regra 10; spec TL, seção 3): cada ação tem um nome estável, um
//! rótulo e no máximo um atalho, e é chamada igual pelo menu, pelo teclado e pelos testes.

use crate::keys::{Key, Shortcut};

mod table;

/// Menu onde o comando aparece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Spread,
    Layout,
    View,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    NewAlbum,
    Open,
    Save,
    SaveAs,
    Export,
    FindMissing,
    Specification,
    Presets,
    Undo,
    Redo,
    NextSpread,
    PreviousSpread,
    FirstSpread,
    LastSpread,
    NextLayout,
    PreviousLayout,
    NextLeft,
    PreviousLeft,
    NextRight,
    PreviousRight,
    Mirror,
    Favorite,
    Reject,
    ToggleReordered,
    ClearFrame,
    Deselect,
    BuildFromSelection,
    NewSpreadFromSelection,
    NewSpread,
    DeleteSpread,
    MoveSpreadLeft,
    MoveSpreadRight,
    CropMode,
    ZoomIn,
    ZoomOut,
    ResetCrop,
    ToggleGuides,
    ToggleSpreads,
    ToggleTimeline,
    TogglePhotos,
    About,
    NewText,
    NewPhotoFrame,
    BringForward,
    SendBackward,
    SaveModel,
    Models,
}

/// Nome, rótulo, menu e atalho de um comando.
type Info = (&'static str, &'static str, Menu, Option<Shortcut>);

impl Command {
    pub const ALL: [Command; 47] = [
        Command::NewAlbum,
        Command::Open,
        Command::Save,
        Command::SaveAs,
        Command::Export,
        Command::FindMissing,
        Command::Specification,
        Command::Presets,
        Command::Undo,
        Command::Redo,
        Command::NextSpread,
        Command::PreviousSpread,
        Command::FirstSpread,
        Command::LastSpread,
        Command::NextLayout,
        Command::PreviousLayout,
        Command::NextLeft,
        Command::PreviousLeft,
        Command::NextRight,
        Command::PreviousRight,
        Command::Mirror,
        Command::Favorite,
        Command::Reject,
        Command::ToggleReordered,
        Command::ClearFrame,
        Command::Deselect,
        Command::BuildFromSelection,
        Command::NewSpreadFromSelection,
        Command::NewSpread,
        Command::DeleteSpread,
        Command::MoveSpreadLeft,
        Command::MoveSpreadRight,
        Command::CropMode,
        Command::ZoomIn,
        Command::ZoomOut,
        Command::ResetCrop,
        Command::ToggleGuides,
        Command::ToggleSpreads,
        Command::ToggleTimeline,
        Command::TogglePhotos,
        Command::About,
        Command::NewText,
        Command::NewPhotoFrame,
        Command::BringForward,
        Command::SendBackward,
        Command::SaveModel,
        Command::Models,
    ];

    /// Nome estável, para testes e para a linha de comando.
    pub fn name(self) -> &'static str {
        self.info().0
    }

    /// Rótulo do menu (e do histórico, nas edições).
    pub fn label(self) -> &'static str {
        self.info().1
    }

    pub fn menu(self) -> Menu {
        self.info().2
    }

    pub fn shortcut(self) -> Option<Shortcut> {
        self.info().3
    }

    /// O comando do atalho, se houver. Ctrl+Shift+Z também refaz (costume no macOS e no Linux), e
    /// Backspace também tira a foto (o Delete do Mac).
    pub fn from_shortcut(shortcut: Shortcut) -> Option<Command> {
        if shortcut == Shortcut::ctrl_shift('z') {
            return Some(Command::Redo);
        }
        // Sinais e números: o Shift depende do teclado (o + pede Shift no ABNT2), então não conta.
        let shortcut = match shortcut.key {
            Key::Char(c) if !c.is_alphabetic() => Shortcut {
                shift: false,
                ..shortcut
            },
            _ => shortcut,
        };
        if shortcut == Shortcut::key(Key::Backspace) {
            return Some(Command::ClearFrame);
        }
        if shortcut == Shortcut::key(Key::Char('=')) {
            return Some(Command::ZoomIn);
        }
        Command::ALL
            .into_iter()
            .find(|c| c.shortcut() == Some(shortcut))
    }

    /// O comando de nome `name`, se houver.
    pub fn from_name(name: &str) -> Option<Command> {
        Command::ALL.into_iter().find(|c| c.name() == name)
    }
}
