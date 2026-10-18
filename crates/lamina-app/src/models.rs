//! Modelos do fotógrafo pela janela (spec MD): o nome ao salvar a lâmina como modelo e a lista
//! "Meus modelos" (renomear, apagar). As decisões são do editor.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, ModelUi};

type Shared = Rc<RefCell<App>>;

/// O aviso de nome repetido já foi mostrado para este nome: o segundo clique substitui (regra 1).
fn confirmed(ui: &ModelUi<'_>, name: &str) -> bool {
    ui.get_name_message()
        .starts_with(&format!("Já existe um modelo \"{name}\""))
}

/// Lâmina → Salvar como modelo…
pub fn open_save(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    ui.set_name("".into());
    ui.set_name_message("".into());
    ui.set_show_name(true);
}

fn name_save(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    let name = ui.get_name().trim().to_owned();
    if app.editor.model_exists(&name) && !confirmed(&ui, &name) {
        ui.set_name_message(
            format!("Já existe um modelo \"{name}\". Clique em Salvar de novo para substituir.")
                .into(),
        );
        return;
    }
    match app.editor.save_model(&name) {
        Ok(()) => {
            ui.set_show_name(false);
            app.notice = format!("Modelo \"{name}\" salvo.");
            app.refresh_current();
            w.invoke_focus_main();
        }
        Err(e) => ui.set_name_message(e.to_string().into()),
    }
}

fn refresh_list(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let names: Vec<SharedString> = app
        .editor
        .models()
        .iter()
        .map(|m| m.name.clone().into())
        .collect();
    w.global::<ModelUi>().set_models(model(names));
}

/// Lâmina → Meus modelos…
pub fn open_list(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    refresh_list(app);
    let ui = w.global::<ModelUi>();
    ui.set_list_selected(-1);
    ui.set_list_name("".into());
    ui.set_list_message("".into());
    ui.set_show_list(true);
}

/// O nome selecionado na lista.
fn selected(app: &App, ui: &ModelUi<'_>) -> Option<String> {
    let i = usize::try_from(ui.get_list_selected()).ok()?;
    app.editor.models().get(i).map(|m| m.name.clone())
}

fn list_change(app: &mut App, rename: bool) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    let Some(old) = selected(app, &ui) else {
        return;
    };
    let result = if rename {
        app.editor.rename_model(&old, &ui.get_list_name())
    } else {
        app.editor.remove_model(&old)
    };
    match result {
        Ok(_) => {
            ui.set_list_message("".into());
            ui.set_list_selected(-1);
            ui.set_list_name("".into());
            refresh_list(app);
            app.refresh_current();
        }
        Err(e) => ui.set_list_message(e.to_string().into()),
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<ModelUi>();
    let a = app.clone();
    ui.on_name_save(move || name_save(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_list_rename(move || list_change(&mut a.borrow_mut(), true));
    let a = app.clone();
    ui.on_list_delete(move || list_change(&mut a.borrow_mut(), false));
    let w = window.as_weak();
    ui.on_closed(move || {
        if let Some(w) = w.upgrade() {
            w.invoke_focus_main();
        }
    });
}
