//! Os nomes dos tamanhos prontos (spec PR e ET, correções de 2026-10-29): o nome é a medida,
//! "largura x altura" em cm ("30x30cm"), e um nome repetido entre encadernadoras se acha pelo
//! nome completo, "25x20cm (ProAlbuns)".

use crate::Preset;

/// "30x30cm", "20x15cm" ou "21,5x30cm": largura × altura da página, em cm.
pub fn size_name(width_cm: f64, height_cm: f64) -> String {
    format!("{}x{}cm", number(width_cm), number(height_cm))
}

/// "30" ou "21,5", como nos outros números da etapa 1.
fn number(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    format!("{rounded}").replace('.', ",")
}

/// As medidas escritas num nome "LxAcm", para conferir que o nome bate com o preset (ET-B10).
pub fn parse_size_name(name: &str) -> Option<(f64, f64)> {
    let (w, h) = name.strip_suffix("cm")?.split_once('x')?;
    let read = |t: &str| t.replace(',', ".").parse::<f64>().ok();
    Some((read(w)?, read(h)?))
}

impl Preset {
    /// O nome com a encadernadora, quando há: "25x20cm (ProAlbuns)" (PR-B13).
    pub fn full_name(&self) -> String {
        match &self.binder {
            Some(binder) => format!("{} ({binder})", self.name),
            None => self.name.clone(),
        }
    }
}

/// O preset pelo nome completo ou, se não houver, pelo nome sozinho (o primeiro, que é o do
/// Lamina quando o nome se repete), sem ligar para maiúsculas (PR-B13).
pub fn find_preset<'a>(presets: &'a [Preset], name: &str) -> Option<&'a Preset> {
    let name = name.trim();
    presets
        .iter()
        .find(|p| p.full_name().eq_ignore_ascii_case(name))
        .or_else(|| presets.iter().find(|p| p.name.eq_ignore_ascii_case(name)))
}

/// O nome de antes da correção de 2026-10-29 e a encadernadora gravados no último tamanho
/// (`ultimo_tamanho`) viram o nome de agora (ET-B12). `None` quando o nome já é o de agora.
pub fn renamed(name: &str, binder: Option<&str>) -> Option<(String, Option<String>)> {
    if let Some(rest) = name.strip_prefix("ProAlbuns ") {
        // A ProAlbuns escrevia altura x largura.
        let (h, w) = rest.split_once('x')?;
        return Some((format!("{w}x{h}cm"), binder.map(str::to_owned)));
    }
    let rest = ["Quadrado ", "Horizontal ", "Vertical "]
        .iter()
        .find_map(|prefix| name.strip_prefix(prefix))?;
    let (w, h) = rest.split_once(" × ")?;
    Some((format!("{w}x{h}cm"), binder.map(str::to_owned)))
}
