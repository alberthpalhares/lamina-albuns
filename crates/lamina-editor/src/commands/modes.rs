//! Atalhos por modo (spec EG, regra 2): cada comando diz em que modo da lâmina vale, e nenhum
//! atalho se repete dentro do mesmo modo. No modo organizar as setas movem a seleção e + e − dão
//! zoom na lâmina; fora dele, as setas trocam o layout e + e − são o zoom do recorte.

use super::Command;
use crate::keys::{Key, Shortcut};

/// O modo da lâmina grande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Organize,
}

impl Command {
    /// O comando vale no modo `mode`?
    pub fn in_mode(self, mode: Mode) -> bool {
        use Command as C;
        match self {
            // Só fora do modo: as setas trocam o layout; Enter, F, X e R; o zoom do recorte.
            C::NextLayout
            | C::PreviousLayout
            | C::NextLeft
            | C::PreviousLeft
            | C::NextRight
            | C::PreviousRight
            | C::Favorite
            | C::Reject
            | C::ToggleReordered
            | C::BuildFromSelection
            | C::NewSpreadFromSelection
            | C::ZoomIn
            | C::ZoomOut => mode == Mode::Normal,
            // Só no modo: mover a seleção, selecionar todos, o zoom da lâmina, Alinhar, Igualar e
            // Distribuir.
            C::NudgeLeft
            | C::NudgeRight
            | C::NudgeUp
            | C::NudgeDown
            | C::NudgeLeftMore
            | C::NudgeRightMore
            | C::NudgeUpMore
            | C::NudgeDownMore
            | C::SelectAllFrames
            | C::ViewZoomIn
            | C::ViewZoomOut
            | C::ViewFit
            | C::AlignLeft
            | C::AlignCenter
            | C::AlignRight
            | C::AlignTop
            | C::AlignMiddle
            | C::AlignBottom
            | C::EqualWidth
            | C::EqualHeight
            | C::EqualSize
            | C::DistributeAcross
            | C::DistributeDown => mode == Mode::Organize,
            _ => true,
        }
    }

    /// O comando do atalho no modo `mode`. Ctrl+Shift+Z também refaz (costume no macOS e no
    /// Linux), Backspace também tira a foto (o Delete do Mac) e = vale o +.
    pub fn from_shortcut_in(shortcut: Shortcut, mode: Mode) -> Option<Command> {
        if shortcut == Shortcut::ctrl_shift('z') {
            return Some(Command::Redo);
        }
        // Sinais e números: o Shift depende do teclado (o + pede Shift no ABNT2), então não conta.
        let mut shortcut = match shortcut.key {
            Key::Char(c) if !c.is_alphabetic() => Shortcut {
                shift: false,
                ..shortcut
            },
            _ => shortcut,
        };
        if shortcut == Shortcut::key(Key::Backspace) {
            shortcut = Shortcut::key(Key::Delete);
        }
        if shortcut == Shortcut::key(Key::Char('=')) {
            shortcut = Shortcut::key(Key::Char('+'));
        }
        Command::ALL
            .into_iter()
            .find(|c| c.in_mode(mode) && c.shortcut() == Some(shortcut))
    }

    /// O comando do atalho fora do modo organizar.
    pub fn from_shortcut(shortcut: Shortcut) -> Option<Command> {
        Command::from_shortcut_in(shortcut, Mode::Normal)
    }
}
