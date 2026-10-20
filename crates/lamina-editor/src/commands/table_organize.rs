//! A tabela do registro, parte 3: os comandos do modo organizar a lâmina (spec EG).

use super::{Command, Info, Menu};
use crate::keys::{Key, Shortcut};

impl Command {
    /// Nome estável, rótulo, menu e atalho dos comandos do modo organizar.
    pub(super) fn organize_info(self) -> Option<Info> {
        use Command as C;
        use Shortcut as S;
        let k = |key| Some(S::key(key));
        let shift = |key| Some(S::shift(key));
        Some(match self {
            C::Organize => (
                "lamina.organizar",
                "Organizar lâmina",
                Menu::Spread,
                k(Key::Char('o')),
            ),
            C::SelectAllFrames => (
                "editar.selecionar-quadros",
                "Selecionar todos os quadros",
                Menu::Edit,
                Some(S::ctrl('a')),
            ),
            C::NudgeLeft => (
                "organizar.esquerda",
                "Mover 1 mm para a esquerda",
                Menu::Edit,
                k(Key::Left),
            ),
            C::NudgeRight => (
                "organizar.direita",
                "Mover 1 mm para a direita",
                Menu::Edit,
                k(Key::Right),
            ),
            C::NudgeUp => (
                "organizar.cima",
                "Mover 1 mm para cima",
                Menu::Edit,
                k(Key::Up),
            ),
            C::NudgeDown => (
                "organizar.baixo",
                "Mover 1 mm para baixo",
                Menu::Edit,
                k(Key::Down),
            ),
            C::NudgeLeftMore => (
                "organizar.esquerda-10",
                "Mover 10 mm para a esquerda",
                Menu::Edit,
                shift(Key::Left),
            ),
            C::NudgeRightMore => (
                "organizar.direita-10",
                "Mover 10 mm para a direita",
                Menu::Edit,
                shift(Key::Right),
            ),
            C::NudgeUpMore => (
                "organizar.cima-10",
                "Mover 10 mm para cima",
                Menu::Edit,
                shift(Key::Up),
            ),
            C::NudgeDownMore => (
                "organizar.baixo-10",
                "Mover 10 mm para baixo",
                Menu::Edit,
                shift(Key::Down),
            ),
            C::ViewZoomIn => (
                "organizar.mais-zoom",
                "Aproximar a lâmina",
                Menu::View,
                k(Key::Char('+')),
            ),
            C::ViewZoomOut => (
                "organizar.menos-zoom",
                "Afastar a lâmina",
                Menu::View,
                k(Key::Char('-')),
            ),
            C::ViewFit => ("organizar.ajustar", "Ajustar a lâmina", Menu::View, None),
            _ => return None,
        })
    }
}
