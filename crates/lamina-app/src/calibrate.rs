//! Visualizar → "Calibrar o tamanho real…" pela janela (spec ZR, regra 8): o diálogo do cartão
//! com o monitor da janela; "Guardar" e "Voltar à estimativa" são do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::calibration::{CARD_MM, MAX_PX_PER_MM, MIN_PX_PER_MM};
use slint::ComponentHandle;

use crate::state::App;
use crate::{AppWindow, CalibrateUi};

type Shared = Rc<RefCell<App>>;

/// Abre o diálogo com o cartão no tamanho de agora (o calibrado ou o estimado).
pub fn open(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    crate::monitor::read(app, w.window());
    let ui = w.global::<CalibrateUi>();
    let monitor = app.editor.monitor().clone();
    // A largura em pixels lógicos fica entre 1 e 20 pixels físicos por mm.
    #[allow(clippy::cast_possible_truncation)]
    let logical = |physical: f64| (physical * CARD_MM / monitor.scale) as f32;
    ui.set_monitor(monitor.describe().into());
    ui.set_min_width(logical(MIN_PX_PER_MM));
    ui.set_max_width(logical(MAX_PX_PER_MM));
    #[allow(clippy::cast_possible_truncation)]
    ui.set_width(app.editor.card_width() as f32);
    ui.set_calibrated(app.editor.calibrated());
    ui.set_show(true);
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<CalibrateUi>();
    let a = app.clone();
    ui.on_save(move |width| {
        let mut app = a.borrow_mut();
        if app.editor.calibrate(f64::from(width)) {
            close(&mut app);
        }
    });
    let a = app.clone();
    ui.on_forget(move || {
        let mut app = a.borrow_mut();
        app.editor.forget_calibration();
        close(&mut app);
    });
}

fn close(app: &mut App) {
    if let Some(w) = app.window.upgrade() {
        w.global::<CalibrateUi>().set_show(false);
        w.invoke_focus_main();
    }
    app.refresh_current();
}
