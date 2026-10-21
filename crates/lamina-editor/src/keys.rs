//! Teclas e atalhos do registro de comandos (spec JP, regra 10).

/// Uma tecla, sem os modificadores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Letra ou número, sempre em minúscula.
    Char(char),
    PageDown,
    PageUp,
    Home,
    End,
    Left,
    Right,
    Up,
    Down,
    Delete,
    Backspace,
    Escape,
    Enter,
    /// A ajuda (spec AT: os atalhos do teclado).
    F1,
}

impl Key {
    /// A tecla pelo nome que a janela manda: "PageDown", "Home"… ou o próprio caractere.
    pub fn from_name(name: &str) -> Option<Key> {
        let key = match name {
            "PageDown" => Key::PageDown,
            "PageUp" => Key::PageUp,
            "Home" => Key::Home,
            "End" => Key::End,
            "Left" => Key::Left,
            "Right" => Key::Right,
            "Up" => Key::Up,
            "Down" => Key::Down,
            "Delete" => Key::Delete,
            "Backspace" => Key::Backspace,
            "Escape" => Key::Escape,
            "Enter" => Key::Enter,
            "F1" => Key::F1,
            _ => {
                let mut chars = name.chars();
                let c = chars.next()?;
                if chars.next().is_some() || c.is_control() {
                    return None;
                }
                Key::Char(c.to_lowercase().next().unwrap_or(c))
            }
        };
        Some(key)
    }
}

/// Tecla com modificadores. `ctrl` é Ctrl no Windows e no Linux e Cmd no macOS; `alt` é Alt
/// (Option no macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl Shortcut {
    pub const fn key(key: Key) -> Self {
        Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key,
        }
    }

    pub const fn shift(key: Key) -> Self {
        Shortcut {
            ctrl: false,
            shift: true,
            alt: false,
            key,
        }
    }

    pub const fn ctrl_key(key: Key) -> Self {
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key,
        }
    }

    pub const fn ctrl(c: char) -> Self {
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Key::Char(c),
        }
    }

    pub const fn ctrl_shift(c: char) -> Self {
        Shortcut {
            ctrl: true,
            shift: true,
            alt: false,
            key: Key::Char(c),
        }
    }

    /// Alt com uma tecla (faixas da janela, spec PN: Ctrl+1/2/3 batem com o ZoomIt).
    pub const fn alt(c: char) -> Self {
        Shortcut {
            ctrl: false,
            shift: false,
            alt: true,
            key: Key::Char(c),
        }
    }

    /// Texto para o menu: "Ctrl+Shift+S", "Page Down".
    pub fn text(&self) -> String {
        let key = match self.key {
            Key::Char(c) => c.to_uppercase().to_string(),
            Key::PageDown => "Page Down".into(),
            Key::PageUp => "Page Up".into(),
            Key::Home => "Home".into(),
            Key::End => "End".into(),
            Key::Left => "←".into(),
            Key::Right => "→".into(),
            Key::Up => "↑".into(),
            Key::Down => "↓".into(),
            Key::Delete => "Delete".into(),
            Key::Backspace => "Backspace".into(),
            Key::Escape => "Esc".into(),
            Key::Enter => "Enter".into(),
            Key::F1 => "F1".into(),
        };
        let ctrl = if self.ctrl { "Ctrl+" } else { "" };
        let shift = if self.shift { "Shift+" } else { "" };
        let alt = if self.alt { "Alt+" } else { "" };
        format!("{ctrl}{alt}{shift}{key}")
    }
}
