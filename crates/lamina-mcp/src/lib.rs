//! # lamina-mcp
//!
//! O controle do Lamina por IA pelo MCP (spec IA; ADR 0010). Este crate só traduz: as
//! ferramentas, a execução e as recusas moram no `lamina-editor` ([`lamina_editor::agent`]), e o
//! que sai daqui é o protocolo, por stdio, sem rede. O `tokio` e o `rmcp` ficam presos aqui; os
//! binários só chamam [`serve_stdio`].

use std::path::PathBuf;

use lamina_editor::agent::ToolError;

mod convert;
mod prompts;
mod resources;
mod server;

pub use server::Server;

/// Como o servidor foi ligado (`lamina mcp [álbum] [--permitir pasta]… [--sem-imagens]`).
#[derive(Debug, Clone, Default)]
pub struct ServeOptions {
    /// Álbum aberto ao começar; a pasta dele passa a ser permitida.
    pub album: Option<PathBuf>,
    /// As pastas de `--permitir`.
    pub allow: Vec<PathBuf>,
    /// `false` com `--sem-imagens`.
    pub images: bool,
}

/// Por que o servidor não começou ou parou.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error("O servidor de IA parou: {0}")]
    Stopped(String),
}

/// Atende o cliente de IA pelo stdio até ele fechar. Só o protocolo sai no stdout (regra 11).
pub fn serve_stdio(options: ServeOptions) -> Result<(), McpError> {
    let server = Server::for_photographer(&options)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| McpError::Stopped(e.to_string()))?;
    runtime.block_on(async {
        let running = rmcp::serve_server(server, rmcp::transport::stdio())
            .await
            .map_err(|e| McpError::Stopped(e.to_string()))?;
        running
            .waiting()
            .await
            .map_err(|e| McpError::Stopped(e.to_string()))?;
        Ok(())
    })
}

#[cfg(test)]
mod tests;
