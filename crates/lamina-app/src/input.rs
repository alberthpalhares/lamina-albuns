//! Entrada: os comandos do registro (menu, botões e teclado) e a tradução das teclas especiais.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::frame_form::page_at;
use lamina_editor::{Command, Key, Shortcut};

use crate::state::{App, Pending};
use crate::{AppWindow, export, files, frame_panel, models, relink, spec_dialog};

type Shared = Rc<RefCell<App>>;

/// Roda um comando do registro (menu, botões e teclado passam por aqui).
pub fn run_command(app: &Shared, command: Command) {
    let mut a = app.borrow_mut();
    // O menu também obedece: no meio de um trabalho em segundo plano, nada além de "Sobre".
    if !a.editor.available(command) {
        return;
    }
    match command {
        Command::NewAlbum => files::request(&mut a, Pending::NewAlbum),
        Command::Open => files::request(&mut a, Pending::Open),
        Command::Save => {
            files::save(&mut a, false);
        }
        Command::SaveAs => {
            files::save(&mut a, true);
        }
        Command::Export => export::start(&mut a),
        Command::FindMissing => relink::find(&mut a),
        Command::Specification => spec_dialog::open_spec(&mut a),
        Command::Presets => spec_dialog::open_list(&mut a),
        Command::About => {
            if let Some(w) = a.window.upgrade() {
                w.set_show_about(true);
            }
        }
        Command::NewText => new_text(&mut a),
        Command::SaveModel => models::open_save(&a),
        Command::Models => models::open_list(&a),
        _ => {
            // Navegar e as guias só mudam a lâmina mostrada; o resto edita o álbum e muda também
            // a faixa, a timeline e os usos no navegador.
            let only_view = matches!(
                command,
                Command::NextSpread
                    | Command::PreviousSpread
                    | Command::FirstSpread
                    | Command::LastSpread
                    | Command::ToggleGuides
                    | Command::ToggleSpreads
                    | Command::ToggleTimeline
                    | Command::TogglePhotos
            );
            if a.editor.run(command) {
                if only_view {
                    a.refresh_current();
                } else {
                    a.refresh_all();
                }
            }
        }
    }
}

/// T: texto novo na página sob o mouse, com o cursor no campo do texto (spec TX, regra 11).
fn new_text(app: &mut App) {
    let fx = app.window.upgrade().map_or(-1.0, |w| w.get_hover_fx());
    let side = app
        .editor
        .session()
        .and_then(|s| page_at(s.project(), f64::from(fx)));
    if app.editor.new_text(side) {
        app.refresh_all();
        frame_panel::focus_text(app);
    }
}

/// O nome da tecla especial (o Slint manda um caractere próprio para cada uma) ou o próprio texto.
fn key_name(text: &str) -> String {
    use slint::platform::Key as K;
    const NAMES: [(K, &str); 12] = [
        (K::PageDown, "PageDown"),
        (K::PageUp, "PageUp"),
        (K::Home, "Home"),
        (K::End, "End"),
        (K::LeftArrow, "Left"),
        (K::RightArrow, "Right"),
        (K::Delete, "Delete"),
        (K::Backspace, "Backspace"),
        (K::Escape, "Escape"),
        (K::Return, "Enter"),
        (K::UpArrow, "Up"),
        (K::DownArrow, "Down"),
    ];
    let mut chars = text.chars();
    if let (Some(c), None) = (chars.next(), chars.next())
        && let Some((_, name)) = NAMES.iter().find(|(k, _)| char::from(*k) == c)
    {
        return (*name).to_owned();
    }
    text.to_owned()
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_command(move |name| {
        if let Some(command) = Command::from_name(&name) {
            run_command(&a, command);
        }
    });
    let a = app.clone();
    window.on_key(move |name, ctrl, shift, alt| {
        let Some(key) = Key::from_name(&key_name(&name)) else {
            return false;
        };
        let shortcut = Shortcut {
            ctrl,
            shift,
            alt,
            key,
        };
        // Alt+setas movem o quadro selecionado (spec TX); sem quadro, seguem como atalho.
        let nudged = a.borrow_mut().editor.nudge_key(shortcut);
        if nudged {
            a.borrow_mut().refresh_all();
            return true;
        }
        let Some(command) = Command::from_shortcut(shortcut) else {
            return false;
        };
        if a.borrow().editor.available(command) {
            run_command(&a, command);
        }
        true
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teclas_especiais_viram_o_nome_e_o_resto_fica() {
        use slint::platform::Key as K;
        for (key, name) in [
            (K::PageDown, "PageDown"),
            (K::LeftArrow, "Left"),
            (K::Return, "Enter"),
            (K::Escape, "Escape"),
        ] {
            assert_eq!(key_name(&char::from(key).to_string()), name);
            assert!(Key::from_name(&key_name(&char::from(key).to_string())).is_some());
        }
        assert_eq!(key_name("g"), "g");
        assert_eq!(key_name("1"), "1");
    }
}
