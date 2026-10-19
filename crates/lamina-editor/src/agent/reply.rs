//! A resposta de uma ferramenta (spec IA, seção 4) e o andamento das demoradas (regra 10).

use serde_json::Value;

/// Uma imagem da resposta: PNG (prévias) ou JPEG (miniaturas).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolImage {
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

/// A resposta de uma ferramenta: a frase em português, os dados e as imagens permitidas.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolReply {
    pub text: String,
    pub data: Value,
    pub images: Vec<ToolImage>,
}

impl ToolReply {
    pub(super) fn new(text: impl Into<String>, data: Value) -> Self {
        ToolReply {
            text: text.into(),
            data,
            images: Vec::new(),
        }
    }
}

/// Andamento das ferramentas demoradas: (feitos, total).
pub type Progress<'a> = &'a (dyn Fn(usize, usize) + Sync);
