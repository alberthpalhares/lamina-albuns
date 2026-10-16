//! Intervalo de lâminas a exportar (spec EX, regra 4): `"1, 2, 5-7"`.

use crate::ExportError;

/// Lê o intervalo e devolve as lâminas pedidas (começando em 0), sem repetir, na ordem do álbum.
pub fn parse_range(text: &str, total: usize) -> Result<Vec<usize>, ExportError> {
    let bad = || ExportError::BadRange(text.trim().to_owned());
    let mut chosen = vec![false; total];
    for part in text.split(',').map(str::trim) {
        let (first, last) = match part.split_once('-') {
            Some((a, b)) => (number(a).ok_or_else(bad)?, number(b).ok_or_else(bad)?),
            None => {
                let n = number(part).ok_or_else(bad)?;
                (n, n)
            }
        };
        if first == 0 || first > last {
            return Err(bad());
        }
        if last > total {
            return Err(ExportError::RangeTooBig {
                asked: last,
                count: total,
            });
        }
        for flag in &mut chosen[first - 1..last] {
            *flag = true;
        }
    }
    Ok((0..total).filter(|&i| chosen[i]).collect())
}

/// Só algarismos: `"+3"` e `" "` não são números de lâmina.
fn number(text: &str) -> Option<usize> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeros_e_faixas() {
        assert_eq!(parse_range("1, 3-4", 5).unwrap(), [0, 2, 3]);
        assert_eq!(parse_range(" 5 ,1-2,2 ", 5).unwrap(), [0, 1, 4]);
    }

    #[test]
    fn intervalo_mal_escrito() {
        for text in ["a", "", "3-1", "0", "1,,2", "2-", "+3"] {
            assert!(
                matches!(parse_range(text, 5), Err(ExportError::BadRange(_))),
                "{text:?}"
            );
        }
    }

    #[test]
    fn lamina_que_nao_existe() {
        assert!(matches!(
            parse_range("2-9", 5),
            Err(ExportError::RangeTooBig { asked: 9, count: 5 })
        ));
        // Faixa enorme é recusada antes de qualquer conta.
        assert!(matches!(
            parse_range("1-999999999", 5),
            Err(ExportError::RangeTooBig {
                asked: 999_999_999,
                count: 5
            })
        ));
    }
}
