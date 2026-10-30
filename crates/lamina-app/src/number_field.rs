//! As setas ↑ e ↓ nos campos de número (spec NU): a janela pergunta ao editor o texto novo.

use lamina_editor::number_step::{Step, step_number};
use slint::ComponentHandle;

use crate::{AppWindow, NumberStep};

pub fn connect(window: &AppWindow) {
    window
        .global::<NumberStep>()
        .on_apply(|text, times, step, min, max| {
            let step = Step {
                step: f64::from(step),
                min: f64::from(min),
                max: f64::from(max),
            };
            step_number(&text, times, step).map_or(text, Into::into)
        });
}
