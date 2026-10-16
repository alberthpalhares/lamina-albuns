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

/// Tecla com modificadores. `ctrl` é Ctrl no Windows e no Linux e Cmd no macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub ctrl: bool,
    pub shift: bool,
    pub key: Key,
}

impl Shortcut {
    pub const fn key(key: Key) -> Self {
        Shortcut {
            ctrl: false,
            shift: false,
            key,
        }
    }

    pub const fn shift(key: Key) -> Self {
        Shortcut {
            ctrl: false,
            shift: true,
            key,
        }
    }

    pub const fn ctrl(c: char) -> Self {
        Shortcut {
            ctrl: true,
            shift: false,
            key: Key::Char(c),
        }
    }

    pub const fn ctrl_shift(c: char) -> Self {
        Shortcut {
            ctrl: true,
            shift: true,
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
        };
        let ctrl = if self.ctrl { "Ctrl+" } else { "" };
        let shift = if self.shift { "Shift+" } else { "" };
        format!("{ctrl}{shift}{key}")
    }
}
