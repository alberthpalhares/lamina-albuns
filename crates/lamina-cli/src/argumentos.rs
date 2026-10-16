//! Erros de digitação nos argumentos em português (spec EI, regra 10): o `clap` só fala inglês,
//! então o tipo do erro e o argumento viram a nossa mensagem, com "Erro: " e código 1 como os
//! outros erros do `lamina`. Ajuda e versão saem como o `clap` escreve.

use std::process::ExitCode;

use clap::error::{ContextKind, ContextValue, ErrorKind};

/// Mostra o erro do `clap` e devolve o código de saída.
pub fn report(err: &clap::Error) -> ExitCode {
    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            // Sem terminal para escrever, não há a quem avisar.
            let _ = err.print();
            ExitCode::SUCCESS
        }
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            let _ = err.print();
            ExitCode::FAILURE
        }
        _ => {
            eprintln!("Erro: {}", message(err));
            ExitCode::FAILURE
        }
    }
}

/// A mensagem em português: "opção desconhecida: --laminass.".
pub fn message(err: &clap::Error) -> String {
    let arg = context(err, ContextKind::InvalidArg);
    match err.kind() {
        ErrorKind::UnknownArgument => format!("opção desconhecida: {arg}."),
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => {
            let value = context(err, ContextKind::InvalidValue);
            format!("valor inválido para {arg}: {value}.")
        }
        ErrorKind::MissingRequiredArgument => format!("falta informar {arg}."),
        ErrorKind::InvalidSubcommand => {
            let command = context(err, ContextKind::InvalidSubcommand);
            format!("comando desconhecido: {command}.")
        }
        ErrorKind::MissingSubcommand => "falta o comando; veja lamina --help.".to_owned(),
        _ => "argumentos que não entendi; veja lamina --help.".to_owned(),
    }
}

/// O valor do contexto, sem o marcador de valor ("--laminas <LAMINAS>" → "--laminas"); vários
/// argumentos saem juntos ("--saida e --laminas").
fn context(err: &clap::Error, kind: ContextKind) -> String {
    let names: Vec<String> = match err.get(kind) {
        Some(ContextValue::String(s)) => vec![bare(s)],
        Some(ContextValue::Strings(list)) => list.iter().map(|s| bare(s)).collect(),
        _ => Vec::new(),
    };
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} e {last}", rest.join(", ")),
    }
}

fn bare(text: &str) -> String {
    text.split_whitespace().next().unwrap_or(text).to_owned()
}
