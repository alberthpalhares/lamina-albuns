//! Registro de comandos (spec JP, regra 10; spec TL, seção 3): cada ação tem um nome estável, um
//! rótulo e no máximo um atalho, e é chamada igual pelo menu, pelo teclado e pelos testes.

use crate::keys::{Key, Shortcut};

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
    ToggleGuides,
    About,
}

type Info = (&'static str, &'static str, Menu, Option<Shortcut>);

impl Command {
    pub const ALL: [Command; 22] = [
        Command::NewAlbum,
        Command::Open,
        Command::Save,
        Command::SaveAs,
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
        Command::ToggleGuides,
        Command::About,
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

    fn info(self) -> Info {
        use Command as C;
        use Shortcut as S;
        let k = |key| Some(S::key(key));
        let shift = |key| Some(S::shift(key));
        match self {
            C::NewAlbum => (
                "arquivo.novo",
                "Novo álbum…",
                Menu::File,
                Some(S::ctrl('n')),
            ),
            C::Open => ("arquivo.abrir", "Abrir…", Menu::File, Some(S::ctrl('o'))),
            C::Save => ("arquivo.salvar", "Salvar", Menu::File, Some(S::ctrl('s'))),
            C::SaveAs => (
                "arquivo.salvar-como",
                "Salvar como…",
                Menu::File,
                Some(S::ctrl_shift('s')),
            ),
            C::Undo => (
                "editar.desfazer",
                "Desfazer",
                Menu::Edit,
                Some(S::ctrl('z')),
            ),
            C::Redo => ("editar.refazer", "Refazer", Menu::Edit, Some(S::ctrl('y'))),
            C::NextSpread => (
                "lamina.proxima",
                "Próxima lâmina",
                Menu::Spread,
                k(Key::PageDown),
            ),
            C::PreviousSpread => (
                "lamina.anterior",
                "Lâmina anterior",
                Menu::Spread,
                k(Key::PageUp),
            ),
            C::FirstSpread => (
                "lamina.primeira",
                "Primeira lâmina",
                Menu::Spread,
                k(Key::Home),
            ),
            C::LastSpread => ("lamina.ultima", "Última lâmina", Menu::Spread, k(Key::End)),
            C::NextLayout => (
                "layout.proximo",
                "Próximo layout",
                Menu::Layout,
                k(Key::Right),
            ),
            C::PreviousLayout => (
                "layout.anterior",
                "Layout anterior",
                Menu::Layout,
                k(Key::Left),
            ),
            C::NextLeft => (
                "layout.esquerda-proximo",
                "Próximo layout da página esquerda",
                Menu::Layout,
                k(Key::Down),
            ),
            C::PreviousLeft => (
                "layout.esquerda-anterior",
                "Layout anterior da página esquerda",
                Menu::Layout,
                k(Key::Up),
            ),
            C::NextRight => (
                "layout.direita-proximo",
                "Próximo layout da página direita",
                Menu::Layout,
                shift(Key::Down),
            ),
            C::PreviousRight => (
                "layout.direita-anterior",
                "Layout anterior da página direita",
                Menu::Layout,
                shift(Key::Up),
            ),
            C::Mirror => (
                "layout.espelhar",
                "Espelhar lâmina",
                Menu::Layout,
                k(Key::Char('e')),
            ),
            C::Favorite => (
                "layout.favoritar",
                "Favoritar layout",
                Menu::Layout,
                k(Key::Char('f')),
            ),
            C::Reject => (
                "layout.rejeitar",
                "Rejeitar layout",
                Menu::Layout,
                k(Key::Char('x')),
            ),
            C::ToggleReordered => (
                "layout.reordenadas",
                "Mostrar reordenadas",
                Menu::Layout,
                k(Key::Char('r')),
            ),
            C::ToggleGuides => ("ver.guias", "Mostrar guias", Menu::View, k(Key::Char('g'))),
            C::About => ("ajuda.sobre", "Sobre o Lâmina", Menu::Help, None),
        }
    }

    /// O comando do atalho, se houver. Ctrl+Shift+Z também refaz (costume no macOS e no Linux).
    pub fn from_shortcut(shortcut: Shortcut) -> Option<Command> {
        if shortcut == Shortcut::ctrl_shift('z') {
            return Some(Command::Redo);
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
