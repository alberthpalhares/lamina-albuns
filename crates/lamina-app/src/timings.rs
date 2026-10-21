//! `LAMINA_TEMPOS=1` (spec MV, seção 4): liga o medidor do editor aos quadros desenhados e escreve
//! as linhas no terminal. As contas são do `lamina_editor::motion::Timings`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use lamina_editor::motion::{BURST_GAP, TIMINGS_VAR, Timings};
use slint::{RenderingState, Timer, TimerMode};

use crate::state::App;

/// O medidor e o instante em que a janela abriu (a origem dos tempos).
#[derive(Debug, Clone)]
pub struct Clock {
    timings: Rc<RefCell<Timings>>,
    origin: Instant,
}

impl Default for Clock {
    fn default() -> Self {
        Clock {
            timings: Rc::new(RefCell::new(Timings::from_value(None))),
            origin: Instant::now(),
        }
    }
}

impl Clock {
    /// Lê a variável; ligada, recebe cada quadro desenhado e encerra as rajadas pelo `Timer`
    /// devolvido (que precisa ficar vivo). Cada quadro desenhado também move a porta dos envios do
    /// rolo (spec RL, regra 9).
    pub fn install(window: &slint::Window) -> (Clock, Option<Timer>) {
        let value = std::env::var(TIMINGS_VAR).ok();
        let clock = Clock {
            timings: Rc::new(RefCell::new(Timings::from_value(value.as_deref()))),
            origin: Instant::now(),
        };
        let on = clock.timings.borrow().is_on();
        let c = clock.clone();
        let mut drawing: Option<Duration> = None;
        let notified = window.set_rendering_notifier(move |state, _| match state {
            RenderingState::BeforeRendering => drawing = Some(c.now()),
            RenderingState::AfterRendering => {
                let end = c.now();
                let start = drawing.take().unwrap_or(end);
                if on {
                    c.write(|t| t.frame(start, end));
                }
                crate::mailbox::later(Duration::ZERO, crate::roll::frame_drawn);
            }
            _ => {}
        });
        if notified.is_err() {
            eprintln!("Lamina: este desenho da janela não avisa os quadros (rolo e {TIMINGS_VAR})");
        }
        if !on {
            return (clock, None);
        }
        let c = clock.clone();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, BURST_GAP / 2, move || {
            let now = c.now();
            c.write(|t| t.tick(now));
        });
        (clock, Some(timer))
    }

    /// O tempo desde que a janela abriu.
    pub fn now(&self) -> Duration {
        self.origin.elapsed()
    }

    fn write(&self, f: impl FnOnce(&mut Timings) -> Vec<String>) {
        // Ocupado só se o desenho acontecer no meio de um comando; aí o quadro fica sem medida.
        if let Ok(mut t) = self.timings.try_borrow_mut() {
            for line in f(&mut t) {
                eprintln!("{line}");
            }
        }
    }

    fn mark(&self, f: impl FnOnce(&mut Timings, Duration)) {
        let now = self.now();
        if let Ok(mut t) = self.timings.try_borrow_mut() {
            f(&mut t, now);
        }
    }
}

impl App {
    /// Roda e mede um comando: `edit` muda o editor e diz se algo mudou; se mudou, `show` leva a
    /// mudança para a tela. Devolve o que `edit` disse.
    pub fn timed(
        &mut self,
        name: &str,
        edit: impl FnOnce(&mut App) -> bool,
        show: impl FnOnce(&mut App),
    ) -> bool {
        let clock = self.clock.clone();
        clock.mark(|t, at| t.begin(name, at));
        let changed = edit(self);
        clock.mark(Timings::editor_done);
        if changed {
            show(self);
        }
        clock.mark(Timings::screen_done);
        changed
    }
}
