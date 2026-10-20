//! A seção Foto pela janela (spec AF, seção 3): os controles chamam o editor, que decide; a grade
//! fina do recorte some 1 s depois do último giro.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_core::FrameContent;
use lamina_editor::editor::AdjustChange;
use slint::{ComponentHandle, Timer, TimerMode};

use crate::state::App;
use crate::{AppWindow, PhotoUi};

type Shared = Rc<RefCell<App>>;

/// Quanto a grade fina fica depois do último giro (spec AF, seção 4).
const FINE_GRID: Duration = Duration::from_secs(1);

thread_local! {
    static SETTLE: Timer = Timer::default();
}

/// Aplica a mudança e redesenha a lâmina; um giro rearma o fim da grade fina.
fn change(app: &Shared, change: AdjustChange) {
    let turned = matches!(change, AdjustChange::Angle(_) | AdjustChange::Turn(_));
    let mut a = app.borrow_mut();
    if a.editor.adjust(change) {
        // A miniatura da faixa também mostra a foto ajustada.
        a.refresh_strip();
        a.refresh_current();
    }
    drop(a);
    if turned {
        settle_later(app);
    }
}

/// Depois de um giro: a grade fina some 1 s depois, se nada girar de novo.
pub fn settle_later(app: &Shared) {
    let a = app.clone();
    SETTLE.with(|t| {
        t.start(TimerMode::SingleShot, FINE_GRID, move || {
            let mut app = a.borrow_mut();
            app.editor.angle_settled();
            app.refresh_current();
        });
    });
}

/// Um número da tela em inteiro (os controles vão de −100 a 100, ou de −450 a 450 décimos).
fn whole(v: f32) -> i32 {
    // Os controles ficam entre −450 e 450, então cabe em i32.
    #[allow(clippy::cast_possible_truncation)]
    let n = v.round() as i32;
    n
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PhotoUi>();
    let a = app.clone();
    ui.on_angle_moved(move |v| change(&a, AdjustChange::Angle(whole(v * 10.0))));
    let a = app.clone();
    ui.on_angle_step(move |t| change(&a, AdjustChange::Turn(t)));
    let a = app.clone();
    ui.on_angle_zero(move || change(&a, AdjustChange::Angle(0)));
    let a = app.clone();
    ui.on_brightness_moved(move |v| change(&a, AdjustChange::Brightness(whole(v))));
    let a = app.clone();
    ui.on_contrast_moved(move |v| change(&a, AdjustChange::Contrast(whole(v))));
    let a = app.clone();
    ui.on_bw_toggled(move |on| change(&a, AdjustChange::Bw(on)));
    let a = app.clone();
    ui.on_restore(move || change(&a, AdjustChange::Restore));
    let a = app.clone();
    ui.on_released(move || a.borrow_mut().editor.end_adjust());
    let (a, w) = (app.clone(), window.as_weak());
    ui.on_angle_accepted(move || {
        let Some(w) = w.upgrade() else {
            return;
        };
        let text = w.global::<PhotoUi>().get_angle();
        let result = a.borrow_mut().editor.set_angle_text(&text);
        typed(&a, &w, result);
        settle_later(&a);
    });
    let (a, w) = (app.clone(), window.as_weak());
    ui.on_scale_accepted(move || {
        let Some(w) = w.upgrade() else {
            return;
        };
        let text = w.global::<PhotoUi>().get_scale();
        let result = a.borrow_mut().editor.set_scale_text(&text);
        typed(&a, &w, result);
    });
}

/// Um campo digitado: a mensagem do editor (já humana) ou o redesenho.
fn typed(app: &Shared, w: &AppWindow, result: Result<bool, &'static str>) {
    let ui = w.global::<PhotoUi>();
    match result {
        Err(message) => ui.set_error(message.into()), // texto já humano
        Ok(_) => {
            ui.set_error("".into());
            let mut a = app.borrow_mut();
            a.refresh_strip();
            a.refresh_current();
        }
    }
}

/// Os valores da foto selecionada nos controles.
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PhotoUi>();
    let Some(adjust) = app.editor.photo_adjust() else {
        ui.set_has_photo(false);
        return;
    };
    ui.set_has_photo(true);
    ui.set_can_turn(app.editor.can_adjust(AdjustChange::Angle(0)));
    let degrees = adjust.degrees();
    ui.set_angle(format!("{}°", lamina_editor::frame_form::decimal(degrees)).into());
    #[allow(clippy::cast_possible_truncation)]
    let to_f32 = |v: f64| v as f32;
    ui.set_angle_value(to_f32(degrees));
    ui.set_brightness(to_f32(f64::from(adjust.brightness)));
    ui.set_contrast(to_f32(f64::from(adjust.contrast)));
    ui.set_bw(adjust.bw);
    ui.set_scale(
        app.editor
            .photo_scale()
            .map(lamina_editor::frame_form::decimal)
            .unwrap_or_default()
            .into(),
    );
    ui.set_note(shared_note(app).into());
}

/// "Vale para esta foto em todos os quadros" quando a foto está em mais de um quadro do álbum.
fn shared_note(app: &App) -> &'static str {
    let (Some(s), Some(frame)) = (app.editor.session(), app.editor.selected_frame()) else {
        return "";
    };
    let project = s.project();
    let Some(FrameContent::Photo { photo, .. }) = project
        .spreads
        .get(s.current())
        .and_then(|sp| sp.frames.get(frame))
        .map(|f| &f.content)
    else {
        return "";
    };
    let uses = project
        .spreads
        .iter()
        .flat_map(|sp| &sp.frames)
        .filter(|f| matches!(&f.content, FrameContent::Photo { photo: p, .. } if p == photo))
        .count();
    if uses > 1 {
        "Vale para esta foto em todos os quadros."
    } else {
        ""
    }
}
