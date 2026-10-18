//! O passo de velocidade da porta (AGENTS.md, regra 13) com uma segunda chance: um teste de
//! velocidade que reprova roda de novo, sozinho, uma vez. `--test-threads=1` protege dos outros
//! testes, mas não da máquina carregada por outros programas (auditoria do Marco 3, M1: 2 de 3
//! rodadas reprovaram e os dois testes passaram logo depois, sozinhos).

use std::process::Command;

const ARGS: [&str; 6] = [
    "test",
    "--workspace",
    "--quiet",
    "--",
    "rapido",
    "--test-threads=1",
];

/// Os testes que reprovaram, pelos cabeçalhos `---- nome stdout ----` da saída do `cargo test`.
pub fn failed_tests(output: &str) -> Vec<String> {
    let mut names: Vec<String> = output
        .lines()
        .filter_map(|l| l.strip_prefix("---- ")?.strip_suffix(" stdout ----"))
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    names
}

fn cargo(args: &[&str]) -> Option<(bool, String)> {
    let out = Command::new(env!("CARGO")).args(args).output().ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    print!("{text}");
    Some((out.status.success(), text))
}

pub fn run() -> bool {
    let Some((ok, output)) = cargo(&ARGS) else {
        return false;
    };
    if ok {
        return true;
    }
    let failed = failed_tests(&output);
    // Sem nome de teste (erro de compilação, por exemplo), não há o que repetir.
    if failed.is_empty() {
        return false;
    }
    eprintln!(
        "\n  ⚠ reprovaram: {}. Rodando de novo, um por vez, sozinhos.",
        failed.join(", ")
    );
    for name in &failed {
        let args = [
            "test",
            "--workspace",
            "--quiet",
            "--",
            "--exact",
            name,
            "--test-threads=1",
        ];
        if !cargo(&args).is_some_and(|(ok, _)| ok) {
            return false;
        }
    }
    eprintln!("  ⚠ passaram na repetição: a máquina estava ocupada. Veja se isso se repete.");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acha_os_testes_que_reprovaram() {
        let out = "running 2 tests\n.F\nfailures:\n\n---- tests::pr::pr_b11_trocar_rapido stdout \
                   ----\nlevou 4s\n\n---- cb_b11_rapido stdout ----\nx\n\nfailures:\n    \
                   tests::pr::pr_b11_trocar_rapido\n";
        assert_eq!(
            failed_tests(out),
            ["cb_b11_rapido", "tests::pr::pr_b11_trocar_rapido"]
        );
        assert!(failed_tests("error[E0425]: não compila").is_empty());
    }
}
