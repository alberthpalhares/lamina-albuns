//! O zoom da lâmina no modo organizar (spec ZR, regras 3 a 7 e 11): Ajustar, vezes (de 1,5 em 1,5
//! até 8×) e o tamanho real; o ponto que fica embaixo do mouse; a vista; e a duração do movimento.
//! Nada disso vai para o arquivo (regra 15).

use super::{Editor, PaletteTool};
use crate::calibration::{Monitor, logical_px_per_mm};

/// Quanto cada passo da lupa, de + e −, e do Ctrl+roda multiplica ou divide (regra 4).
pub const VIEW_STEP: f64 = 1.5;
/// O zoom máximo em vezes.
pub const MAX_VIEW_ZOOM: f64 = 8.0;

/// O zoom da lâmina em destaque.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Zoom {
    /// A lâmina inteira cabe na área (vale 1).
    #[default]
    Fit,
    /// De mais de 1 até 8 vezes Ajustar.
    Times(f64),
    /// O tamanho real (1:1): recalculado a cada vez (regra 7).
    Real,
}

/// O que a tela faz depois de uma mudança de zoom: o fator, a vista (em pixels, a partir do
/// centro da lâmina em Ajustar) e a duração do movimento.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomMove {
    pub factor: f64,
    pub pan: (f64, f64),
    pub ms: u32,
}

/// A vista nova que mantém o ponto `p` (a partir do centro de Ajustar) parado ao ir de `z0` para
/// `z1`: `p·(1 − k) + k·pan`, com `k = z1 / z0` (regra 5).
pub fn zoom_about(p: (f64, f64), pan: (f64, f64), z0: f64, z1: f64) -> (f64, f64) {
    let k = z1 / z0;
    (p.0 * (1.0 - k) + k * pan.0, p.1 * (1.0 - k) + k * pan.1)
}

/// O fator do tamanho real: a lâmina com a sangria (`canvas_mm`) em pixels lógicos do monitor,
/// dividida pela largura de Ajustar (regra 7).
pub fn real_factor(canvas_mm: f64, logical_px_per_mm: f64, fit_px: f64) -> f64 {
    canvas_mm * logical_px_per_mm / fit_px
}

/// Um passo a partir de `now`: aproximar multiplica por 1,5 até 8; afastar divide até Ajustar.
/// Um passo nunca anda no sentido contrário (`None` = nada muda; regra 4).
pub fn step_factor(now: f64, up: bool) -> Option<f64> {
    let next = if up {
        (now * VIEW_STEP).min(MAX_VIEW_ZOOM)
    } else {
        (now / VIEW_STEP).max(1.0)
    };
    let forward = if up {
        next > now + 1e-9
    } else {
        next < now - 1e-9
    };
    forward.then_some(next)
}

/// "1,5×", "2,3×", "8×": uma casa, com vírgula.
pub fn times_label(factor: f64) -> String {
    let tenths = (factor * 10.0).round() / 10.0;
    format!("{tenths}×").replace('.', ",")
}

/// Ajustar ou vezes, pelo fator.
pub(super) fn of_factor(factor: f64) -> Zoom {
    if factor <= 1.0 + 1e-9 {
        Zoom::Fit
    } else {
        Zoom::Times(factor)
    }
}

impl Editor {
    /// O zoom da lâmina no modo (1 = Ajustar).
    pub fn view_zoom(&self) -> f64 {
        match self.organize.zoom {
            Zoom::Fit => 1.0,
            Zoom::Times(f) => f,
            Zoom::Real => self.real_now(),
        }
    }

    /// O zoom do modo, como guardado.
    pub fn zoom(&self) -> Zoom {
        self.organize.zoom
    }

    /// A vista da lâmina no modo, em pixels a partir do centro de Ajustar.
    pub fn view_pan(&self) -> (f64, f64) {
        self.organize.pan
    }

    /// A mão moveu a vista (a tela acompanha na hora; regra 6).
    pub fn set_view_pan(&mut self, x: f64, y: f64) {
        self.organize.pan = (x, y);
    }

    /// O rótulo do zoom: "Ajustar", "1,5×", "1:1" ou "≈ 1:1" sem calibração (regras 3 e 9).
    pub fn zoom_label(&self) -> String {
        match self.organize.zoom {
            Zoom::Fit => "Ajustar".to_owned(),
            Zoom::Times(f) => times_label(f),
            Zoom::Real if self.calibrated() => "1:1".to_owned(),
            Zoom::Real => "≈ 1:1".to_owned(),
        }
    }

    /// A largura de Ajustar, sem a animação (`OrganizeUi.fit-width`; regra 7).
    pub fn set_fit_width(&mut self, px: f64) {
        if px > 0.0 {
            self.view.fit_px = px;
        }
    }

    /// O monitor da janela mudou (regra 10). `true` quando a chave ou a escala mudaram.
    pub fn set_monitor(&mut self, monitor: Monitor) -> bool {
        if self.view.monitor == monitor {
            return false;
        }
        self.view.monitor = monitor;
        true
    }

    /// O monitor da janela.
    pub fn monitor(&self) -> &Monitor {
        &self.view.monitor
    }

    /// O monitor da janela tem calibração?
    pub fn calibrated(&self) -> bool {
        self.strips.calibration(&self.view.monitor.key).is_some()
    }

    /// O fator do 1:1 agora, com a calibração do monitor ou a estimativa.
    fn real_now(&self) -> f64 {
        let Some(s) = self.session.as_ref() else {
            return 1.0;
        };
        let canvas = s.project().spec.canvas_rect().width.get();
        let calibrated = self.strips.calibration(&self.view.monitor.key);
        let per_mm = logical_px_per_mm(calibrated, self.view.monitor.scale);
        real_factor(canvas, per_mm, self.view.fit_px)
    }

    /// Um passo de zoom em volta do ponto `p` (`None` = o centro da área): a lupa, + e −, e o
    /// Ctrl+roda (`animated` = falso). `None` quando nada muda (regra 4).
    pub fn zoom_step(
        &mut self,
        up: bool,
        p: Option<(f64, f64)>,
        animated: bool,
    ) -> Option<ZoomMove> {
        if !self.organizing() {
            return None;
        }
        let now = self.view_zoom();
        let next = step_factor(now, up)?;
        self.zoom_to(of_factor(next), p.unwrap_or((0.0, 0.0)), animated)
    }

    /// O tamanho real (1:1), em volta do centro da área.
    pub fn zoom_real(&mut self) -> Option<ZoomMove> {
        if !self.organizing() {
            return None;
        }
        self.zoom_to(Zoom::Real, (0.0, 0.0), true)
    }

    /// Ajustar: a lâmina inteira, com a vista no centro (regra 11).
    pub fn zoom_fit(&mut self) -> Option<ZoomMove> {
        if !self.organizing() {
            return None;
        }
        self.zoom_to(Zoom::Fit, (0.0, 0.0), true)
    }

    pub(super) fn zoom_to(
        &mut self,
        zoom: Zoom,
        p: (f64, f64),
        animated: bool,
    ) -> Option<ZoomMove> {
        let z0 = self.view_zoom();
        let before = (self.organize.zoom, self.organize.pan);
        self.organize.zoom = zoom;
        let z1 = self.view_zoom();
        // Chegar a Ajustar volta a vista ao centro (regra 4).
        self.organize.pan = if zoom == Zoom::Fit {
            (0.0, 0.0)
        } else {
            zoom_about(p, self.organize.pan, z0, z1)
        };
        if (self.organize.zoom, self.organize.pan) == before {
            return None;
        }
        let ms = self.zoom_move(animated).ms;
        self.organize.ms = ms;
        Some(self.zoom_move(animated))
    }

    /// A duração do último movimento do zoom e da vista (a saída do modo também anima).
    pub fn view_ms(&self) -> u32 {
        self.organize.ms
    }

    /// A ferramenta da paleta (regra 2).
    pub fn tool(&self) -> PaletteTool {
        self.organize.tool
    }

    /// Escolhe a ferramenta (a paleta; Quadro e Texto voltam a Selecionar depois de desenhar).
    pub fn set_tool(&mut self, tool: PaletteTool) {
        self.organize.tool = tool;
    }

    /// Z: a lupa e, com ela escolhida, de volta a Selecionar.
    pub(super) fn toggle_loupe(&mut self) -> bool {
        if !self.organizing() {
            return false;
        }
        self.organize.tool = if self.organize.tool == PaletteTool::Magnifier {
            PaletteTool::Select
        } else {
            PaletteTool::Magnifier
        };
        true
    }

    /// O zoom e a vista de agora, com a duração do movimento (zero com "Reduzir movimento" ou
    /// num gesto à mão; regra 6).
    pub fn zoom_move(&self, animated: bool) -> ZoomMove {
        let ms = if animated {
            self.durations().navigate
        } else {
            0
        };
        ZoomMove {
            factor: self.view_zoom(),
            pan: self.organize.pan,
            ms,
        }
    }
}

impl Editor {
    /// "Guardar" no diálogo do cartão: o cartão com `card_width` pixels lógicos, na escala do
    /// monitor da janela, vira a calibração dele e vai para `janela.json` (regra 8). `false` fora
    /// de 1 a 20 pixels por mm; se não gravar, vale até fechar o Lamina, com o aviso.
    pub fn calibrate(&mut self, card_width: f64) -> bool {
        let px = crate::calibration::card_px_per_mm(card_width, self.view.monitor.scale);
        let key = self.view.monitor.key.clone();
        if !self.strips.set_calibration(&key, px) {
            return false;
        }
        self.save_strips();
        true
    }

    /// "Voltar à estimativa": tira só a calibração do monitor da janela.
    pub fn forget_calibration(&mut self) -> bool {
        let key = self.view.monitor.key.clone();
        if !self.strips.forget_calibration(&key) {
            return false;
        }
        self.save_strips();
        true
    }

    /// A largura do cartão, em pixels lógicos, que o diálogo começa mostrando: a da calibração do
    /// monitor ou a da estimativa.
    pub fn card_width(&self) -> f64 {
        let calibrated = self.strips.calibration(&self.view.monitor.key);
        logical_px_per_mm(calibrated, self.view.monitor.scale) * crate::calibration::CARD_MM
    }
}
