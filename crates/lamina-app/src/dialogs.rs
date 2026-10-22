//! Os diálogos de arquivo do sistema sem prender a linha da janela (falha da beta.5 no Mac).
//!
//! O diálogo síncrono roda um laço de eventos próprio dentro do evento do winit que o abriu; no
//! Mac, um evento que chega nesse meio-tempo (arrastar fotos, por exemplo) faz o winit tratar um
//! evento dentro do outro, e o Lamina fechava. Aqui o diálogo abre preso à janela do Lamina (no
//! Mac, como uma folha) e a resposta volta depois, já fora do evento, com o estado da janela.

use std::future::Future;

use rfd::AsyncFileDialog;
use slint::ComponentHandle;
use slint::winit_030::WinitWindowAccessor;

use crate::state::App;

/// Um diálogo com o título, preso à janela do Lamina quando ela existe.
pub fn dialog(app: &App, title: &str) -> AsyncFileDialog {
    let mut dialog = Some(AsyncFileDialog::new().set_title(title));
    if let Some(w) = app.window.upgrade() {
        w.window().with_winit_window(|window| {
            dialog = dialog.take().map(|d| d.set_parent(window));
        });
    }
    dialog.unwrap_or_else(|| AsyncFileDialog::new().set_title(title))
}

/// Espera a resposta do diálogo e roda `then` com o estado da janela.
pub fn then<T: 'static>(
    answer: impl Future<Output = T> + 'static,
    then: impl FnOnce(&mut App, T) + 'static,
) {
    // Sem laço de eventos (janela fechando), não há a quem responder.
    let _ = slint::spawn_local(async move {
        let value = answer.await;
        crate::mailbox::with_app(move |app| then(app, value));
    });
}
