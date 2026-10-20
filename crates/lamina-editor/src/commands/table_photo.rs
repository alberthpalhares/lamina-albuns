//! A tabela do registro, parte 4: os comandos da foto (spec AF). No modo recorte, `[` e `]` giram
//! 0,1°; com Shift, 1° (o Shift do `[` e do `]` dá `{` e `}`, no teclado dos EUA e no ABNT2).

use super::{Command, Info, Menu};
use crate::keys::{Key, Shortcut};

impl Command {
    /// Nome estável, rótulo, menu e atalho dos comandos da foto.
    pub(super) fn photo_info(self) -> Option<Info> {
        use Command as C;
        let k = |c| Some(Shortcut::key(Key::Char(c)));
        Some(match self {
            C::TurnLeft => (
                "recorte.girar-esquerda",
                "Endireitar −0,1°",
                Menu::Edit,
                k('['),
            ),
            C::TurnRight => (
                "recorte.girar-direita",
                "Endireitar +0,1°",
                Menu::Edit,
                k(']'),
            ),
            C::TurnLeftMore => (
                "recorte.girar-esquerda-1",
                "Endireitar −1°",
                Menu::Edit,
                k('{'),
            ),
            C::TurnRightMore => (
                "recorte.girar-direita-1",
                "Endireitar +1°",
                Menu::Edit,
                k('}'),
            ),
            _ => return None,
        })
    }
}
