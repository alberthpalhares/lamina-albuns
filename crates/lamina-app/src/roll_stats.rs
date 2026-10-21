//! `LAMINA_TEMPOS=1` no rolo (spec RL, seção 4): cada rolagem ou deslize escreve uma linha no
//! terminal, com os quadros por segundo, o maior quadro, as imagens pedidas e prontas e a memória
//! das imagens guardadas; e quanto as vizinhas levaram para ter a imagem de tela depois de navegar.

use std::time::Duration;

use lamina_editor::motion::{TIMINGS_VAR, Timings};

/// A medição do rolo; desligada, não guarda nada.
#[derive(Debug)]
pub struct RollStats {
    on: bool,
    frames: Vec<Duration>,
    asked: usize,
    painted: Vec<f64>,
    /// Quando foi a última navegação, até as vizinhas à vista ficarem nítidas.
    navigated: Option<Duration>,
}

impl Default for RollStats {
    fn default() -> Self {
        let value = std::env::var(TIMINGS_VAR).ok();
        RollStats {
            on: Timings::from_value(value.as_deref()).is_on(),
            frames: Vec::new(),
            asked: 0,
            painted: Vec::new(),
            navigated: None,
        }
    }
}

impl RollStats {
    pub fn asked(&mut self, n: usize) {
        self.asked = self.asked.max(n);
    }

    pub fn painted(&mut self, ms: f64) {
        if self.on {
            self.painted.push(ms);
        }
    }

    /// Um quadro desenhado em `at`; `moving`: o rolo está rolando ou deslizando. Quando ele para,
    /// a linha sai (com `bytes` de imagens guardadas).
    pub fn frame(&mut self, at: Duration, moving: bool, bytes: usize) {
        if !self.on {
            return;
        }
        if moving {
            self.frames.push(at);
            return;
        }
        if self.frames.len() < 2 {
            self.frames.clear();
            return;
        }
        eprintln!("{}", self.line(bytes));
        self.frames.clear();
        self.painted.clear();
        self.asked = 0;
    }

    fn line(&self, bytes: usize) -> String {
        let span = self.frames.last().copied().unwrap_or_default()
            - self.frames.first().copied().unwrap_or_default();
        #[allow(clippy::cast_precision_loss)]
        let fps = (self.frames.len() - 1) as f64 / span.as_secs_f64().max(1e-6);
        let worst = self
            .frames
            .windows(2)
            .map(|w| w[1] - w[0])
            .max()
            .unwrap_or_default();
        let (lo, hi) = self
            .painted
            .iter()
            .fold((f64::MAX, 0.0_f64), |(a, b), v| (a.min(*v), b.max(*v)));
        let range = if self.painted.is_empty() {
            String::new()
        } else {
            format!(" ({lo:.0}–{hi:.0} ms)")
        };
        #[allow(clippy::cast_precision_loss)]
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!(
            "rolo: {fps:.0} q/s · maior quadro {} ms · imagens: {} pedidas, {} prontas{range} · {mb:.0} MB",
            worst.as_millis(),
            self.asked,
            self.painted.len()
        )
    }

    /// Navegou em `at`: começa a contar até as vizinhas à vista ficarem nítidas.
    pub fn navigate(&mut self, at: Duration) {
        if self.on {
            self.navigated = Some(at);
        }
    }

    /// As vizinhas à vista têm a imagem de tela: escreve quanto levou desde a navegação.
    pub fn sharp(&mut self, at: Duration) {
        if let Some(t) = self.navigated.take() {
            eprintln!(
                "rolo: vizinhas nítidas em {} ms",
                at.saturating_sub(t).as_millis()
            );
        }
    }

    /// Esperando as vizinhas ficarem nítidas.
    pub fn waiting(&self) -> bool {
        self.navigated.is_some()
    }
}
