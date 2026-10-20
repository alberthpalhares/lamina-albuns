//! A medição para a equipe (spec MV, seção 4): com `LAMINA_TEMPOS=1`, cada comando diz quanto
//! levou no editor, na tela e até o primeiro quadro, e cada rajada de quadros diz quantos
//! desenhou e o que a começou. Os instantes chegam como tempo desde a abertura da janela, para
//! esta conta não depender do relógio.

use std::time::Duration;

/// A variável de ambiente que liga a medição (valor `1`).
pub const TIMINGS_VAR: &str = "LAMINA_TEMPOS";

/// Intervalo sem nenhum quadro que encerra uma rajada.
pub const BURST_GAP: Duration = Duration::from_millis(100);

/// Comando que não desenhou nada depois deste tempo é relatado assim mesmo.
const NO_FRAME_AFTER: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
struct Measured {
    name: String,
    start: Duration,
    editor: Option<Duration>,
    screen: Option<Duration>,
}

#[derive(Debug, Clone)]
struct Burst {
    cause: Option<String>,
    first: Duration,
    last: Duration,
    frames: usize,
    max_gap: Duration,
    max_draw: Duration,
}

/// Os comandos e os quadros medidos. Desligada, não guarda nada e não escreve nada.
#[derive(Debug, Default)]
pub struct Timings {
    on: bool,
    command: Option<Measured>,
    burst: Option<Burst>,
}

impl Timings {
    /// Ligada só quando a variável vale `1`.
    pub fn from_value(value: Option<&str>) -> Self {
        Timings {
            on: value.is_some_and(|v| v.trim() == "1"),
            ..Timings::default()
        }
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// Um comando começou em `at`.
    pub fn begin(&mut self, name: &str, at: Duration) {
        if self.on {
            self.command = Some(Measured {
                name: name.to_owned(),
                start: at,
                editor: None,
                screen: None,
            });
        }
    }

    /// O editor terminou o comando.
    pub fn editor_done(&mut self, at: Duration) {
        if let Some(c) = &mut self.command {
            c.editor = Some(at);
        }
    }

    /// A tela terminou de receber o que mudou.
    pub fn screen_done(&mut self, at: Duration) {
        if let Some(c) = &mut self.command {
            c.screen = Some(at);
        }
    }

    /// Um quadro desenhado entre `start` e `end`. Devolve as linhas para escrever: a rajada que
    /// acabou e a do comando cujo primeiro quadro é este.
    pub fn frame(&mut self, start: Duration, end: Duration) -> Vec<String> {
        if !self.on {
            return Vec::new();
        }
        let mut lines = Vec::new();
        let command = self.command.take();
        let continues = self
            .burst
            .as_ref()
            .is_some_and(|b| start.saturating_sub(b.last) <= BURST_GAP);
        if command.is_some() || !continues {
            lines.extend(self.burst.take().map(|b| burst_line(&b)));
        }
        if let Some(c) = &command {
            lines.push(command_line(c, Some(end)));
        }
        let draw = end.saturating_sub(start);
        match &mut self.burst {
            Some(b) => {
                b.max_gap = b.max_gap.max(end.saturating_sub(b.last));
                b.last = end;
                b.frames += 1;
                b.max_draw = b.max_draw.max(draw);
            }
            None => {
                self.burst = Some(Burst {
                    cause: command.map(|c| c.name),
                    first: end,
                    last: end,
                    frames: 1,
                    max_gap: Duration::ZERO,
                    max_draw: draw,
                });
            }
        }
        lines
    }

    /// Chamada de tempos em tempos: encerra a rajada parada há mais de [`BURST_GAP`] e relata o
    /// comando que não desenhou nada.
    pub fn tick(&mut self, now: Duration) -> Vec<String> {
        let mut lines = Vec::new();
        if self
            .burst
            .as_ref()
            .is_some_and(|b| now.saturating_sub(b.last) > BURST_GAP)
        {
            lines.extend(self.burst.take().map(|b| burst_line(&b)));
        }
        if self
            .command
            .as_ref()
            .is_some_and(|c| now.saturating_sub(c.start) > NO_FRAME_AFTER)
        {
            lines.extend(self.command.take().map(|c| command_line(&c, None)));
        }
        lines
    }
}

/// Milissegundos com uma casa e vírgula.
fn ms(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f64() * 1000.0).replace('.', ",")
}

fn command_line(c: &Measured, first: Option<Duration>) -> String {
    let since =
        |t: Option<Duration>| t.map_or_else(|| "—".to_owned(), |t| ms(t.saturating_sub(c.start)));
    let editor = since(c.editor);
    let screen = match (c.editor, c.screen) {
        (Some(e), Some(s)) => ms(s.saturating_sub(e)),
        _ => "—".to_owned(),
    };
    let first = first.map_or_else(
        || "nenhum".to_owned(),
        |t| format!("{} ms", ms(t.saturating_sub(c.start))),
    );
    format!(
        "{}: editor {editor} ms · tela {screen} ms · 1º quadro {first}",
        c.name
    )
}

fn burst_line(b: &Burst) -> String {
    let span = b.last.saturating_sub(b.first);
    let rate = if b.frames > 1 && !span.is_zero() {
        format!("{:.0} q/s", (b.frames - 1) as f64 / span.as_secs_f64())
    } else {
        "—".to_owned()
    };
    let cause = b.cause.as_deref().map_or_else(
        || "sem comando (mouse, rolagem ou desenho parado)".to_owned(),
        |c| format!("depois de {c}"),
    );
    format!(
        "  {} quadro(s) em {} ms ({rate}), maior intervalo {} ms, desenho até {} ms · {cause}",
        b.frames,
        ms(span),
        ms(b.max_gap),
        ms(b.max_draw)
    )
}
