//! As setas ↑ e ↓ nos campos de número (spec NU): a conta do passo, igual em todos os campos.
//! A janela só pergunta o texto novo.

/// O passo de um campo e a faixa que ele aceita (regras 1 e 3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub step: f64,
    pub min: f64,
    pub max: f64,
}

/// O texto depois de andar `times` passos (positivo sobe; Shift manda 10). `None` quando o texto
/// não é um número: o campo fica como está (regra 5). Mantém a vírgula ou o ponto do texto e as
/// casas do passo ou do texto, o que for maior (regra 4).
pub fn step_number(text: &str, times: i32, step: Step) -> Option<String> {
    let trimmed = text.trim();
    // A unidade escrita junto ("12,0°", "150%") fica no fim, como estava.
    let number_end = trimmed
        .rfind(|c: char| c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let (number, unit) = trimmed.split_at(number_end);
    let typographic = number.starts_with(MINUS);
    let comma = !number.contains('.');
    let value: f64 = number.replace(MINUS, "-").replace(',', ".").parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    let places = decimals(number).max(step_places(step.step));
    let next = (value + f64::from(times) * step.step).clamp(step.min, step.max);
    let mut out = format!("{next:.places$}");
    // "-0" não aparece para o fotógrafo.
    if out
        .trim_start_matches('-')
        .chars()
        .all(|c| c == '0' || c == '.')
    {
        out = out.trim_start_matches('-').to_owned();
    }
    if comma {
        out = out.replace('.', ",");
    }
    if typographic {
        out = out.replace('-', &MINUS.to_string());
    }
    Some(out + unit)
}

/// O sinal de menos que o Lamina escreve nos ângulos ("−12,0°").
const MINUS: char = '\u{2212}';

/// As casas do passo, com folga: o passo vem da janela em `f32`, e 0,1 chega como 0,10000000149.
fn step_places(step: f64) -> usize {
    (0..6)
        .find(|n| {
            let scaled = step * 10f64.powi(*n);
            (scaled - scaled.round()).abs() < 1e-4
        })
        .map_or(6, |n| usize::try_from(n).unwrap_or(6))
}

/// Quantas casas depois da vírgula ou do ponto.
fn decimals(text: &str) -> usize {
    text.split(['.', ','])
        .nth(1)
        .map_or(0, |d| d.chars().take_while(char::is_ascii_digit).count())
}
