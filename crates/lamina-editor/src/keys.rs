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

    /// Texto para o menu: "Ctrl+Shift+S", "Page Down" (o de Windows e Linux).
    pub fn text(&self) -> String {
        self.text_on(Platform::Windows)
    }

    /// Texto no sistema `platform` (spec AT, regra 5): no Mac, "⇧⌘S", "⌥1", "⌘⌦". `{` e `}` são
    /// Shift+[ e Shift+], como se digitam nos teclados EUA e ABNT2.
    pub fn text_on(&self, platform: Platform) -> String {
        let (key, shift) = match self.key {
            Key::Char('{') => (Key::Char('['), true),
            Key::Char('}') => (Key::Char(']'), true),
            key => (key, self.shift),
        };
        let mac = platform == Platform::Mac;
        let name = key_text(key, mac);
        if mac {
            let alt = if self.alt { "⌥" } else { "" };
            let shift = if shift { "⇧" } else { "" };
            let cmd = if self.ctrl { "⌘" } else { "" };
            format!("{alt}{shift}{cmd}{name}")
        } else {
            let ctrl = if self.ctrl { "Ctrl+" } else { "" };
            let alt = if self.alt { "Alt+" } else { "" };
            let shift = if shift { "Shift+" } else { "" };
            format!("{ctrl}{alt}{shift}{name}")
        }
    }
}

/// O nome da tecla sem os modificadores; no Mac, Delete, Backspace e Enter viram símbolos.
fn key_text(key: Key, mac: bool) -> String {
    let text = match key {
        Key::Char(c) => return c.to_uppercase().to_string(),
        Key::Delete if mac => "⌦",
        Key::Backspace if mac => "⌫",
        Key::Enter if mac => "↩",
        Key::PageDown => "Page Down",
        Key::PageUp => "Page Up",
        Key::Home => "Home",
        Key::End => "End",
        Key::Left => "←",
        Key::Right => "→",
        Key::Up => "↑",
        Key::Down => "↓",
        Key::Delete => "Delete",
        Key::Backspace => "Backspace",
        Key::Escape => "Esc",
        Key::Enter => "Enter",
        Key::F1 => "F1",
    };
    text.to_owned()
}

/// O sistema, para o texto das teclas (spec AT, seção 4): Ctrl no Windows e no Linux, ⌘ no Mac.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Mac,
    Linux,
}

impl Platform {
    /// O sistema em que o Lamina roda.
    pub fn current() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::Mac
        } else if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Linux
        }
    }
}
