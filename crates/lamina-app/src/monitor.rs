//! O monitor da janela pelo winit (spec ZR, regra 10): o nome, os pixels e a escala, lidos ao
//! abrir e quando a janela muda de monitor ou de escala, no mesmo gancho dos arquivos soltos (o
//! Slint guarda um gancho por janela). Só uma mudança chega ao editor.

use lamina_editor::calibration::Monitor;
use slint::winit_030::{WinitWindowAccessor, winit};

use crate::state::App;

/// Lê o monitor em que a janela está e passa ao editor; `true` se mudou. Sem monitor (Wayland,
/// às vezes), vale "monitor desconhecido".
pub fn read(app: &mut App, window: &slint::Window) -> bool {
    let scale = f64::from(window.scale_factor());
    let found = window
        .with_winit_window(|w| {
            w.current_monitor()
                .map(|m| (m.name(), (m.size().width, m.size().height)))
        })
        .flatten();
    let monitor = match found {
        Some((name, size)) => Monitor::new(name.as_deref(), size, scale),
        None => Monitor::new(None, (0, 0), scale),
    };
    app.editor.set_monitor(monitor)
}

/// Os eventos do winit que podem trocar o monitor ou a escala.
pub fn may_change(event: &winit::event::WindowEvent) -> bool {
    matches!(
        event,
        winit::event::WindowEvent::Moved(_)
            | winit::event::WindowEvent::ScaleFactorChanged { .. }
            | winit::event::WindowEvent::Resized(_)
    )
}
