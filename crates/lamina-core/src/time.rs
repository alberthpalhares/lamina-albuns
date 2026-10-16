//! Data e hora de captura das fotos.
//!
//! As câmeras gravam a hora local sem fuso ("2026:09:12 15:00:00"). Guardamos os segundos desde
//! 1970-01-01 00:00:00 nesse mesmo relógio local: dá para ordenar e medir intervalos entre fotos
//! da mesma câmera, que é o que o agrupamento por momento precisa.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Instante de captura, em segundos desde 1970-01-01 00:00:00 (hora local da câmera).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct CaptureTime(i64);

impl CaptureTime {
    /// Lê o formato EXIF `AAAA:MM:DD HH:MM:SS`. Datas zeradas ou inválidas viram `None`.
    pub fn from_exif(text: &str) -> Option<Self> {
        let text = text.trim().trim_end_matches('\0');
        let (date, time) = text.split_once(' ')?;
        let mut d = date.split(':').map(str::parse::<i64>);
        let mut t = time.split(':').map(str::parse::<i64>);
        let (year, month, day) = (d.next()?.ok()?, d.next()?.ok()?, d.next()?.ok()?);
        let (hour, minute, second) = (t.next()?.ok()?, t.next()?.ok()?, t.next()?.ok()?);
        Self::from_parts(year, month, day, hour, minute, second)
    }

    pub fn from_parts(year: i64, month: i64, day: i64, h: i64, m: i64, s: i64) -> Option<Self> {
        let valid = (1..=9999).contains(&year)
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && (0..24).contains(&h)
            && (0..60).contains(&m)
            && (0..=60).contains(&s);
        valid.then(|| {
            CaptureTime(days_from_civil(year, month, day) * 86_400 + h * 3600 + m * 60 + s)
        })
    }

    /// Instante a partir dos segundos desde 1970 (o inverso de [`CaptureTime::seconds`]).
    pub fn from_seconds(seconds: i64) -> Self {
        CaptureTime(seconds)
    }

    pub fn seconds(self) -> i64 {
        self.0
    }

    /// Segundos entre este instante e `later` (negativo se `later` vier antes).
    pub fn seconds_until(self, later: CaptureTime) -> i64 {
        later.0 - self.0
    }
}

impl fmt::Display for CaptureTime {
    /// `AAAA-MM-DD HH:MM:SS`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.0.div_euclid(86_400);
        let secs = self.0.rem_euclid(86_400);
        let (y, mo, d) = civil_from_days(days);
        let (h, mi, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
        write!(f, "{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Dias desde 1970-01-01 (algoritmo "days_from_civil" de Howard Hinnant).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverso de [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_o_formato_exif() {
        let t = CaptureTime::from_exif("2026:09:12 15:00:40").unwrap();
        assert_eq!(t.to_string(), "2026-09-12 15:00:40");
    }

    #[test]
    fn epoca_e_zero() {
        assert_eq!(
            CaptureTime::from_parts(1970, 1, 1, 0, 0, 0)
                .unwrap()
                .seconds(),
            0
        );
    }

    #[test]
    fn intervalo_entre_fotos() {
        let a = CaptureTime::from_exif("2026:09:12 15:00:00").unwrap();
        let b = CaptureTime::from_exif("2026:09:12 16:00:40").unwrap();
        assert_eq!(a.seconds_until(b), 3640);
        assert!(a < b);
    }

    #[test]
    fn virada_de_ano_bissexto() {
        let a = CaptureTime::from_exif("2024:02:28 23:59:59").unwrap();
        let b = CaptureTime::from_exif("2024:02:29 00:00:00").unwrap();
        assert_eq!(a.seconds_until(b), 1);
        assert_eq!(b.to_string(), "2024-02-29 00:00:00");
    }

    #[test]
    fn ida_e_volta_por_varios_seculos() {
        for days in [-100_000, -1, 0, 1, 19_000, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }

    #[test]
    fn datas_invalidas_sao_ignoradas() {
        assert_eq!(CaptureTime::from_exif("0000:00:00 00:00:00"), None);
        assert_eq!(CaptureTime::from_exif("2026:02:30 10:00:00"), None);
        assert_eq!(CaptureTime::from_exif("lixo"), None);
        assert_eq!(CaptureTime::from_exif("2026:09:12 25:00:00"), None);
    }

    #[test]
    fn aceita_texto_com_nulo_no_fim() {
        assert!(CaptureTime::from_exif("2026:09:12 15:00:00\0").is_some());
    }
}
