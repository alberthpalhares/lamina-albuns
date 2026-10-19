//! Tradução entre o agente e o protocolo: uma ferramenta do catálogo vira `Tool`, uma resposta
//! vira `CallToolResult` (frase, dados e imagens) e uma recusa vira erro de ferramenta. Só
//! parâmetro errado vira erro de protocolo (spec IA, regra 13).

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use lamina_editor::agent::{ToolError, ToolInfo, ToolReply};
use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock, JsonObject, Tool};
use serde_json::Value;

pub(crate) fn tool(info: ToolInfo) -> Tool {
    let schema = match info.schema {
        Value::Object(map) => map,
        _ => JsonObject::new(),
    };
    Tool::new(info.name, info.description, Arc::new(schema))
}

/// A frase, os dados em JSON (também como texto, para os clientes que não leem os dados
/// estruturados) e as imagens.
pub(crate) fn result(reply: ToolReply) -> CallToolResult {
    let mut content = vec![ContentBlock::text(reply.text)];
    let data = (reply.data.is_object()).then_some(reply.data);
    if let Some(data) = &data {
        content.push(ContentBlock::text(data.to_string()));
    }
    for image in reply.images {
        content.push(ContentBlock::image(encode(&image.bytes), image.mime));
    }
    let mut result = CallToolResult::success(content);
    result.structured_content = data;
    result
}

/// Recusa da ferramenta: o cliente mostra a frase à IA. Parâmetro errado é do protocolo.
pub(crate) fn failure(error: ToolError) -> Result<CallToolResult, ErrorData> {
    match error {
        ToolError::UnknownTool(_) | ToolError::BadArguments(_) => {
            Err(ErrorData::invalid_params(error.to_string(), None))
        }
        other => Ok(CallToolResult::error(vec![ContentBlock::text(
            other.to_string(),
        )])),
    }
}

pub(crate) fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}
