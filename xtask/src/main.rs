//! Tarefas do projeto. Uso: `cargo xtask <tarefa>`.
//!
//! - `verify`: a porta de verificação única (a mesma que o CI roda).
//! - `layers`: confere que nenhum crate depende de uma camada igual ou acima dele.
//! - `size`: alerta de tamanho de arquivo (nunca reprova).
//! - `specs`: toda spec implementada tem um teste por cenário BDD.
//! - `asserts`: nenhuma assertiva frouxa (`is_err()` sem conferir o erro) nos testes.
//! - `sala-limpa`: nada da marca ou dos arquivos do software de referência no produto.
//! - `espelho [--trazer] <pasta>`: copia só o código para o clone do `lamina-codigo` (público).
//! - `portao <termos>`: lista o que foi alocado para a fase que está abrindo.

mod asserts;
mod docs;
mod espelho;
mod layers;
mod portao;
mod sala_limpa;
mod size;
mod specs;

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let ok = match task.as_str() {
        "verify" => verify(),
        "layers" => layers::run(),
        "size" => {
            size::run();
            true
        }
        "specs" => specs::run(),
        "sala-limpa" => sala_limpa::run(),
        "asserts" => asserts::run(),
        "docs" => docs::run(),
        "espelho" => espelho::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        "portao" => portao::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        _ => {
            eprintln!(
                "uso: cargo xtask <verify | layers | size | specs | asserts | sala-limpa | portao>"
            );
            false
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Uma checagem do repositório: nome e função que diz se passou.
type Check = (&'static str, fn() -> bool);

/// Roda todos os passos em ordem e para no primeiro que reprovar.
fn verify() -> bool {
    let cargo_steps: [(&str, &[&str]); 3] = [
        ("formatação", &["fmt", "--all", "--check"]),
        (
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("testes", &["test", "--workspace", "--quiet"]),
    ];
    for (name, args) in cargo_steps {
        if !step(name, || cargo(args)) {
            return false;
        }
    }
    let checks: [Check; 5] = [
        ("camadas", layers::run),
        ("specs", specs::run),
        ("assertivas", asserts::run),
        ("sala limpa", sala_limpa::run),
        ("documentos", docs::run),
    ];
    for (name, check) in checks {
        if !step(name, check) {
            return false;
        }
    }
    size::run();
    if cargo_deny_available() {
        let deny = || cargo(&["deny", "check", "licenses", "bans", "sources"]);
        if !step("licenças (cargo deny)", deny) {
            return false;
        }
    } else {
        println!("\n· licenças: cargo-deny não instalado aqui; o CI confere.");
    }
    println!("\n✔ porta de verificação: tudo verde");
    true
}

fn step(name: &str, run: impl FnOnce() -> bool) -> bool {
    println!("\n▶ {name}");
    let ok = run();
    if !ok {
        eprintln!("\n✘ porta de verificação reprovou em: {name}");
    }
    ok
}

fn cargo(args: &[&str]) -> bool {
    Command::new(env!("CARGO"))
        .args(args)
        .status()
        .is_ok_and(|status| status.success())
}

fn cargo_deny_available() -> bool {
    Command::new(env!("CARGO"))
        .args(["deny", "--version"])
        .output()
        .is_ok_and(|out| out.status.success())
}
