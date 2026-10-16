//! Apoio aos testes de velocidade (AGENTS.md, regra 13): uma folga só, igual em todos os crates.

use std::time::Duration;

/// Até quantas vezes um teste de velocidade mede: a menor medida vale, e ele para assim que fica
/// dentro da meta. Os outros testes do mesmo binário disputam a máquina por alguns segundos; uma
/// lentidão de verdade continua acima da meta em todas as medidas.
pub const TRIES: usize = 15;

/// Meta de tempo da spec, em milissegundos. No CI (variável `CI`), as máquinas são mais lentas e
/// rodam os testes em paralelo: a folga é 3 vezes, o que ainda pega uma lentidão de verdade.
pub fn budget(ms: u64) -> Duration {
    let factor = if std::env::var_os("CI").is_some() {
        3
    } else {
        1
    };
    Duration::from_millis(ms * factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folga_so_no_ci() {
        let expected = if std::env::var_os("CI").is_some() {
            300
        } else {
            100
        };
        assert_eq!(budget(100), Duration::from_millis(expected));
    }
}
