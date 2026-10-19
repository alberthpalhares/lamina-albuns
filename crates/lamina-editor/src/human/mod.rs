//! Mensagens humanas (spec EH): cada erro que chega à janela vira uma frase de fotógrafo, o que
//! fazer agora e o detalhe técnico, que fica em "Mostrar detalhes" e no registro de erros.
//!
//! Cada tipo de erro implementa [`Human`] com um `match` que nomeia toda variante, sem `_ =>`
//! (regra 1 da spec; `cargo xtask erros` confere): uma variante nova sem frase não compila.

mod from_core;
mod from_editor;
mod from_export;
mod from_layout;
mod from_media;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

/// "O que fazer" de quem não tem outra saída além de tentar de novo (regra 3).
pub const REPORT: &str = "Se continuar, use Ajuda → Enviar sugestão ou problema.";

/// Um erro dito para o fotógrafo: a frase, o que fazer e o detalhe técnico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanMessage {
    pub title: String,
    pub what_to_do: String,
    pub details: String,
}

/// Um erro que sabe se explicar ao fotógrafo.
pub trait Human {
    fn human(&self) -> HumanMessage;
}

impl HumanMessage {
    pub fn new(
        title: impl Into<String>,
        what_to_do: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        HumanMessage {
            title: title.into(),
            what_to_do: what_to_do.into(),
            details: details.into(),
        }
    }

    /// A frase e o que fazer na mesma linha: avisos e mensagens abaixo dos campos (seção 4).
    pub fn line(&self) -> String {
        format!("{} {}", self.title, self.what_to_do)
    }

    /// Uma tarefa entrou em pânico (regra 6).
    pub fn unexpected(details: impl Into<String>) -> Self {
        HumanMessage::new(
            "O Lamina teve um problema inesperado.",
            "Salve o álbum, se puder, e abra o Lamina de novo. Se acontecer outra vez, use Ajuda → \
             Enviar sugestão ou problema e anexe o registro.",
            details,
        )
    }
}

/// O detalhe técnico: o texto do erro e as causas, uma por linha, sem repetir (regra 5).
pub fn technical(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(c) = cause {
        let line = c.to_string();
        if !text.contains(&line) {
            text.push('\n');
            text.push_str(&line);
        }
        cause = c.source();
    }
    text
}

/// Roda `work` protegido: um pânico vira a mensagem de falha inesperada (regra 6).
pub fn guarded<T>(work: impl FnOnce() -> T) -> Result<T, HumanMessage> {
    catch_unwind(AssertUnwindSafe(work))
        .map_err(|payload| HumanMessage::unexpected(panic_text(payload.as_ref())))
}

/// Só o nome do arquivo ou da pasta, sem o caminho (regra 2).
pub(crate) fn name(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    Path::new(last)
        .file_name()
        .map_or_else(|| last.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// Erro que só acontece por defeito do programa (regra 7).
pub(crate) fn bug(details: String) -> HumanMessage {
    HumanMessage::new(
        "Algo não saiu como o esperado nesta lâmina.",
        format!("Desfaça com Ctrl+Z e tente de novo. {REPORT}"),
        details,
    )
}

/// "Confira se há espaço…": o que fazer quando gravar falha (seção 1, tabela).
pub(crate) const CHECK_DISK: &str =
    "Confira se há espaço no disco e se a pasta pode ser gravada, e tente de novo.";

/// O texto de um pânico (`panic!("…")` ou com `format!`).
pub(crate) fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "falha sem mensagem".to_owned())
}
