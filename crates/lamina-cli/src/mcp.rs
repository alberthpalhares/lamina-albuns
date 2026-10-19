//! `lamina mcp`: o Lamina controlado por IA pelo MCP, por stdio (spec IA; ADR 0010). O trabalho
//! todo está no `lamina-mcp`; aqui só as opções. Nada mais pode sair no stdout (regra 11).

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use lamina_mcp::{ServeOptions, serve_stdio};

#[derive(Debug, Args)]
pub struct McpArgs {
    /// Álbum (.lamina) aberto ao começar; a pasta dele fica liberada para a IA.
    pub album: Option<PathBuf>,
    /// Pasta que a IA pode ler e gravar (pode repetir).
    #[arg(long = "permitir")]
    pub permitir: Vec<PathBuf>,
    /// Nenhuma imagem vai para a IA: só os dados das fotos e das lâminas.
    #[arg(long = "sem-imagens")]
    pub sem_imagens: bool,
}

pub fn run(args: McpArgs) -> Result<()> {
    serve_stdio(ServeOptions {
        album: args.album,
        allow: args.permitir,
        images: !args.sem_imagens,
    })?;
    Ok(())
}
