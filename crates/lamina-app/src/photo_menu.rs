//! O menu de botão direito da grade de fotos e o Delete com a grade em foco (spec TF): tirar do
//! álbum, com a pergunta quando há lâmina afetada, e mostrar o arquivo no Explorador ou no
//! Finder. As decisões ficam no `lamina-editor` (regra 1).

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::Human;
use lamina_editor::leaving::{leave_label, leave_question};
use lamina_editor::reveal::{System, reveal_command};
use slint::ComponentHandle;

use crate::state::App;
use crate::{AppWindow, PhotoMenuUi};

type Shared = Rc<RefCell<App>>;

/// Os textos do menu seguem a seleção e o sistema (regra 1).
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PhotoMenuUi>();
    ui.set_remove_text(leave_label(app.editor.selection().len().max(1)).into());
    ui.set_reveal_text(System::current().reveal_label().into());
}

/// "Tirar do álbum" ou Delete na grade: pergunta só se alguma foto está em lâmina (regra 4).
pub fn ask_remove(app: &mut App) {
    let photos = app.editor.selection().to_vec();
    let Some(w) = app.window.upgrade() else {
        return;
    };
    match leave_question(app.editor.photos_in_spreads(&photos)) {
        None => remove(app),
        Some(question) => {
            refresh(app);
            let ui = w.global::<PhotoMenuUi>();
            ui.set_question(question.into());
            ui.set_show_ask(true);
        }
    }
}

fn remove(app: &mut App) {
    let photos = app.editor.selection().to_vec();
    if let Some(w) = app.window.upgrade() {
        w.global::<PhotoMenuUi>().set_show_ask(false);
    }
    if app.editor.remove_from_album(&photos) {
        app.project_changed();
    }
}

/// Abre a pasta da primeira foto selecionada, com o arquivo marcado (regra 7).
fn reveal(app: &mut App) {
    let Some(file) = app
        .editor
        .selection()
        .first()
        .and_then(|id| app.editor.photo_file(*id))
    else {
        return;
    };
    match reveal_command(&file, System::current()) {
        Ok((program, args)) => {
            // O Explorador sai com código 1 mesmo quando abre: não há o que conferir.
            let _ = std::process::Command::new(program).args(args).spawn();
        }
        Err(e) => crate::files::show_error(app, &e.human()),
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PhotoMenuUi>();
    let on = |f: fn(&mut App)| {
        let a = app.clone();
        move || f(&mut a.borrow_mut())
    };
    ui.on_remove(on(ask_remove));
    ui.on_confirm(on(remove));
    ui.on_reveal(on(reveal));
    refresh(&app.borrow());
}
