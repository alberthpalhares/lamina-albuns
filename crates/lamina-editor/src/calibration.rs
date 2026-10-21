//! O tamanho real (1:1) do modo organizar (spec ZR, regras 7 a 10): o monitor da janela, a
//! calibração de cada monitor com um cartão de banco e a estimativa sem ela. A calibração fica em
//! `janela.json` (`tamanho_real`), fora do `Strips`.

use serde::{Deserialize, Serialize};

/// A largura de um cartão de banco (ISO/IEC 7810 ID-1), em mm.
pub const CARD_MM: f64 = 85.6;
/// A altura do cartão, em mm.
pub const CARD_HEIGHT_MM: f64 = 53.98;
/// Sem calibração: 96 pontos por polegada vezes a escala, em pixels lógicos por mm (regra 9).
pub const ESTIMATE_PX_PER_MM: f64 = 96.0 / 25.4;
/// A calibração aceita, em pixels físicos por mm (regra 8).
pub const MIN_PX_PER_MM: f64 = 1.0;
pub const MAX_PX_PER_MM: f64 = 20.0;
/// A chave de um monitor que o sistema não informa (Wayland, às vezes).
pub const UNKNOWN_MONITOR: &str = "monitor desconhecido";
/// A dica do 1:1 sem calibração (regra 9).
pub const ESTIMATE_TIP: &str =
    "Tamanho real aproximado: calibre em Visualizar → Calibrar o tamanho real…";

/// O monitor em que a janela está: a chave ("DISPLAY2 3840×2160") e a escala da janela.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    pub key: String,
    pub scale: f64,
}

impl Default for Monitor {
    fn default() -> Self {
        Monitor {
            key: UNKNOWN_MONITOR.to_owned(),
            scale: 1.0,
        }
    }
}

impl Monitor {
    /// O monitor pelo nome que o sistema dá (no Windows, a porta: `\.\DISPLAY2`) e pelos pixels.
    /// Trocar a resolução conta como outro monitor (regra 8).
    pub fn new(name: Option<&str>, size: (u32, u32), scale: f64) -> Self {
        let key = match name.map(|n| n.trim_start_matches(r"\.\").trim()) {
            Some(n) if !n.is_empty() => format!("{n} {}×{}", size.0, size.1),
            _ => UNKNOWN_MONITOR.to_owned(),
        };
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        Monitor { key, scale }
    }

    /// "DISPLAY2 · 3840 × 2160 · escala 150%", para o diálogo de calibração.
    pub fn describe(&self) -> String {
        let shown = self.key.replacen(' ', " · ", 1).replace('×', " × ");
        let percent = (self.scale * 100.0).round();
        format!("{shown} · escala {percent}%")
    }
}

/// A calibração de um monitor, como vai para `janela.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    pub monitor: String,
    /// Pixels físicos do monitor por milímetro.
    #[serde(rename = "px_por_mm")]
    pub px_per_mm: f64,
}

impl Calibration {
    /// Dentro de 1 a 20 pixels físicos por mm (regra 8)?
    pub fn is_valid(&self) -> bool {
        self.px_per_mm.is_finite() && (MIN_PX_PER_MM..=MAX_PX_PER_MM).contains(&self.px_per_mm)
    }
}

/// Os pixels físicos por mm de um cartão desenhado com `width` pixels lógicos na escala `scale`.
pub fn card_px_per_mm(width: f64, scale: f64) -> f64 {
    width * scale / CARD_MM
}

/// Os pixels lógicos por mm: os da calibração (físicos) divididos pela escala, ou a estimativa.
pub fn logical_px_per_mm(calibrated: Option<f64>, scale: f64) -> f64 {
    match calibrated {
        Some(physical) if scale > 0.0 => physical / scale,
        _ => ESTIMATE_PX_PER_MM,
    }
}

/// As calibrações de `janela.json`, sem as de fora de 1 a 20 (são ignoradas e saem na próxima
/// gravação; regra 8). Um campo ilegível vale lista vazia.
pub(crate) fn read_list(text: &str) -> Vec<Calibration> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(list) = value.get("tamanho_real").cloned() else {
        return Vec::new();
    };
    serde_json::from_value::<Vec<Calibration>>(list)
        .unwrap_or_default()
        .into_iter()
        .filter(Calibration::is_valid)
        .collect()
}

impl crate::StripsStore {
    /// A calibração (pixels físicos por mm) do monitor `key`.
    pub fn calibration(&self, key: &str) -> Option<f64> {
        self.calibrations
            .iter()
            .find(|c| c.monitor == key)
            .map(|c| c.px_per_mm)
    }

    /// Guarda (ou troca) a calibração do monitor `key`; as outras nunca mudam. `false` fora de 1
    /// a 20.
    pub fn set_calibration(&mut self, key: &str, px_per_mm: f64) -> bool {
        let new = Calibration {
            monitor: key.to_owned(),
            px_per_mm,
        };
        if !new.is_valid() {
            return false;
        }
        match self.calibrations.iter_mut().find(|c| c.monitor == key) {
            Some(c) => *c = new,
            None => self.calibrations.push(new),
        }
        true
    }

    /// Tira a calibração do monitor `key` ("Voltar à estimativa"). `false` se não havia.
    pub fn forget_calibration(&mut self, key: &str) -> bool {
        let before = self.calibrations.len();
        self.calibrations.retain(|c| c.monitor != key);
        self.calibrations.len() != before
    }
}
