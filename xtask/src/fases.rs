//! Pendências com destino vencido (AGENTS.md, regra 10; auditoria do Marco 3, M11): uma linha de
//! tabela ou de `Alocação:` que aponta para um marco já fechado (com auditoria em
//! `docs/auditorias/`) precisa dizer o que aconteceu: "feito", "sem fase", "realocado" ou
//! "adiado". Linhas de histórico (as correções datadas) não contam.

use std::fs;
use std::path::Path;

const OUTCOMES: [&str; 4] = ["feito", "sem fase", "realocad", "adiad"];

/// Os marcos fechados: os números dos arquivos `*-marco-N.md` de `dir`.
pub fn closed_phases(dir: &Path) -> Vec<u32> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<u32> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".md")?
                .rsplit_once("-marco-")?
                .1
                .parse()
                .ok()
        })
        .collect();
    out.sort_unstable();
    out
}

/// As linhas (1-based) de `text` com destino num marco de `closed` sem dizer o que aconteceu.
pub fn stale_lines(text: &str, closed: &[u32]) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| l.contains('|') || l.contains("Alocação"))
        .filter(|(_, l)| {
            closed.iter().any(|n| {
                let target = format!("→ Marco {n}");
                l.match_indices(&target).any(|(at, _)| {
                    // "→ Marco 1" não pode casar com "→ Marco 10".
                    !l[at + target.len()..].starts_with(|c: char| c.is_ascii_digit())
                })
            })
        })
        .filter(|(_, l)| !OUTCOMES.iter().any(|o| l.contains(o)))
        .map(|(i, _)| i + 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destino_num_marco_fechado_precisa_dizer_o_que_aconteceu() {
        let text = "| PDF | → Marco 3 |\n| Capa | → Marco 3 (feito, spec X) |\n\
                    | Web | → Marco 4 |\n- 2026 — decidido → Marco 3.\n  `Alocação: → Marco 30`\n\
                    | Rosto | → sem fase |";
        assert_eq!(stale_lines(text, &[1, 2, 3]), [1]);
        assert!(stale_lines(text, &[1, 2]).is_empty());
    }

    #[test]
    fn marcos_fechados_vem_das_auditorias() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "2026-10-15-fundacao.md",
            "2026-10-15-marco-1.md",
            "2026-10-16-marco-2.md",
        ] {
            fs::write(dir.path().join(f), "x").unwrap();
        }
        assert_eq!(closed_phases(dir.path()), [1, 2]);
    }
}
