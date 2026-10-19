//! O formulário de sugestões (spec SR, regras 3 e 4) e as mensagens de quando o navegador ou a
//! pasta de registros não abrem. Nada aqui usa a rede: a janela só abre o endereço no navegador.

use std::io;
use std::path::Path;

use crate::human::{HumanMessage, REPORT};

/// O formulário no Tally, sem conta (decisão do dono, 2026-10-19). Trocar de ferramenta é mudar
/// esta linha e os nomes dos campos escondidos.
pub const FEEDBACK_FORM: &str = "https://tally.so/r/a-definir";

/// O endereço com os três campos escondidos, e nada mais (regra 3; SR-B2 trava).
pub fn feedback_address(version: &str, system: &str, channel: &str) -> String {
    format!(
        "{FEEDBACK_FORM}?versao={}&sistema={}&canal={}",
        encode(version),
        encode(system),
        encode(channel)
    )
}

/// Sistema e processador desta máquina, como `windows-x86_64`.
pub fn system_name() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// `beta` ou `estavel`.
pub fn channel_name(beta: bool) -> &'static str {
    if beta { "beta" } else { "estavel" }
}

/// O navegador não abriu: o endereço para copiar (regra 4).
pub fn browser_failed(address: &str) -> HumanMessage {
    HumanMessage::new(
        "Não foi possível abrir o navegador.",
        format!("Copie este endereço no navegador: {address}"),
        format!("O sistema recusou abrir {address}"),
    )
}

/// A pasta de registros não abriu: o caminho para abrir à mão (regra 10).
pub fn logs_failed(folder: &Path, problem: &io::Error) -> HumanMessage {
    HumanMessage::new(
        "Não foi possível abrir a pasta de registros.",
        format!(
            "Abra esta pasta no explorador: {}. {REPORT}",
            folder.display()
        ),
        folder_details(folder, problem),
    )
}

/// Uma pasta (a da exportação, spec EI) não abriu no explorador.
pub fn folder_failed(folder: &Path, problem: &io::Error) -> HumanMessage {
    HumanMessage::new(
        "Não foi possível abrir a pasta.",
        format!("Abra esta pasta no explorador: {}.", folder.display()),
        folder_details(folder, problem),
    )
}

fn folder_details(folder: &Path, problem: &io::Error) -> String {
    format!("{}: {}", folder.display(), lamina_media::io_reason(problem))
}

/// Codificação de endereço: letras, números e `-._~` passam; o resto vira `%XX`.
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
