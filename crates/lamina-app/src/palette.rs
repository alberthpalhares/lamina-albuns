//! A paleta do modo organizar pela janela (spec EG, seção 3; pedido do dono de 2026-10-20): os
//! ícones rodam os comandos do registro, a ferramenta vale para a lâmina e a paleta se destaca
//! numa janela própria. Cada janela tem o seu `PaletteUi`; o Rust mantém os dois iguais.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::Command;
use slint::ComponentHandle;

use crate::state::App;
use crate::{AppWindow, OrganizeUi, PaletteUi, PaletteWindow};

type Shared = Rc<RefCell<App>>;

thread_local! {
    /// A paleta destacada, enquanto existir (fechá-la a devolve para a janela do álbum).
    static FLOATING: RefCell<Option<PaletteWindow>> = const { RefCell::new(None) };
}

/// Liga os botões de um `PaletteUi` (o da janela do álbum ou o da paleta destacada).
fn wire(ui: &PaletteUi, app: &Shared) {
    let a = app.clone();
    ui.on_command(move |name| {
        if let Some(command) = Command::from_name(&name) {
            crate::input::run_command(&a, command);
        }
    });
    let a = app.clone();
    ui.on_pick_tool(move |tool| {
        let app = a.borrow();
        if let Some(w) = app.window.upgrade() {
            w.global::<OrganizeUi>().set_tool(tool);
        }
        refresh(&app);
    });
    let a = app.clone();
    ui.on_detach(move || detach(&a));
    let a = app.clone();
    ui.on_dock(move || dock(&a));
}

pub fn connect(window: &AppWindow, app: &Shared) {
    wire(&window.global::<PaletteUi>(), app);
}

fn detach(app: &Shared) {
    let Ok(palette) = PaletteWindow::new() else {
        return;
    };
    wire(&palette.global::<PaletteUi>(), app);
    let a = app.clone();
    palette.window().on_close_requested(move || {
        dock(&a);
        slint::CloseRequestResponse::HideWindow
    });
    FLOATING.with(|f| *f.borrow_mut() = Some(palette));
    refresh(&app.borrow());
}

fn dock(app: &Shared) {
    if let Some(p) = FLOATING.with(|f| f.borrow_mut().take()) {
        // Fechar uma janela que já sumiu não tem o que avisar.
        let _ = p.hide();
    }
    refresh(&app.borrow());
}

/// Põe a ferramenta, o zoom e o que vale agora nas duas paletas; a destacada aparece só no modo.
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let tool = w.global::<OrganizeUi>().get_tool();
    let zoom = match app.editor.view_zoom() {
        z if z <= 1.0 => "Ajustar".to_owned(),
        z => format!("{z}×"),
    };
    let fill = |ui: PaletteUi, detached: bool| {
        ui.set_tool(tool);
        ui.set_zoom(zoom.as_str().into());
        ui.set_can_align(app.editor.available(Command::AlignLeft));
        ui.set_can_distribute(app.editor.available(Command::DistributeAcross));
        ui.set_detached(detached);
    };
    let detached = FLOATING.with(|f| {
        let floating = f.borrow();
        if let Some(p) = floating.as_ref() {
            fill(p.global::<PaletteUi>(), true);
            // Mostrar ou esconder uma janela pequena não falha de um jeito que dê para tratar.
            let _ = if app.editor.organizing() {
                p.show()
            } else {
                p.hide()
            };
        }
        floating.is_some()
    });
    fill(w.global::<PaletteUi>(), detached);
}
