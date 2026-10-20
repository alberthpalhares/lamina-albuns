//! A unidade de medida da tela (spec NP, regra 3): mm, cm ou polegadas. Só muda a exibição; o
//! projeto continua em mm (AGENTS, regra 4).

/// Mensagem do número inválido (seção 6).
pub const NOT_A_NUMBER: &str = "Use um número, como 30 ou 30,5.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit {
    Mm,
    #[default]
    Cm,
    Inch,
}

impl Unit {
    pub const ALL: [Unit; 3] = [Unit::Mm, Unit::Cm, Unit::Inch];

    /// "mm", "cm", "pol".
    pub fn label(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Cm => "cm",
            Unit::Inch => "pol",
        }
    }

    fn mm_per_unit(self) -> f64 {
        match self {
            Unit::Mm => 1.0,
            Unit::Cm => 10.0,
            Unit::Inch => 25.4,
        }
    }

    /// A medida em mm como o campo mostra nesta unidade: até duas casas, com vírgula.
    pub fn show(self, mm: f64) -> String {
        number(mm / self.mm_per_unit())
    }

    /// O texto do campo, nesta unidade, em mm. Aceita vírgula e ponto.
    pub fn read(self, text: &str) -> Result<f64, &'static str> {
        parse(text).map(|v| v * self.mm_per_unit())
    }
}

/// Número com até duas casas e vírgula: 30 → "30", 11,811 → "11,81".
pub fn number(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    format!("{rounded}").replace('.', ",")
}

/// Um número finito e não negativo, com vírgula ou ponto.
pub fn parse(text: &str) -> Result<f64, &'static str> {
    match text.trim().replace(',', ".").parse::<f64>() {
        Ok(v) if v.is_finite() && v >= 0.0 => Ok(v),
        _ => Err(NOT_A_NUMBER),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unidades_mostram_e_leem_em_mm() {
        assert_eq!(Unit::Cm.show(300.0), "30");
        assert_eq!(Unit::Inch.show(300.0), "11,81");
        assert_eq!(Unit::Mm.show(2.5), "2,5");
        assert!((Unit::Inch.read("12").unwrap() - 304.8).abs() < 1e-9);
        assert_eq!(Unit::Cm.read("30,5"), Ok(305.0));
        assert_eq!(Unit::Cm.read("-1"), Err(NOT_A_NUMBER));
        assert_eq!(Unit::Cm.read(""), Err(NOT_A_NUMBER));
        assert_eq!(Unit::ALL.map(Unit::label), ["mm", "cm", "pol"]);
    }
}
