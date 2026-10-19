//! `lamina mcp`: o Lamina controlado por IA pelo MCP, por stdio (spec IA; ADR 0010). O trabalho
//! todo está no `lamina-mcp`; aqui só as opções. Nada mais pode sair no stdout (regra 11).
//!
//! O pacote do Claude Desktop (`.mcpb`) liga o servidor com `--imagens true|false` e
//! `--permitir` seguido de todas as pastas escolhidas: o Claude Desktop expande a lista de
//! pastas em vários argumentos e escreve a chave de ligar e desligar como `true` ou `false`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use clap::builder::BoolishValueParser;
use lamina_mcp::{ServeOptions, serve_stdio};

#[derive(Debug, Args)]
pub struct McpArgs {
    /// Álbum (.lamina) aberto ao começar; a pasta dele fica liberada para a IA. Vem antes das
    /// pastas de --permitir.
    pub album: Option<PathBuf>,
    /// Pastas que a IA pode ler e gravar (pode repetir, ou listar várias depois de um --permitir).
    #[arg(long = "permitir", num_args = 1..)]
    pub permitir: Vec<PathBuf>,
    /// Nenhuma imagem vai para a IA: só os dados das fotos e das lâminas.
    #[arg(long = "sem-imagens", conflicts_with = "imagens")]
    pub sem_imagens: bool,
    /// Imagens para a IA ligadas ou desligadas (true ou false); desligadas é o mesmo que
    /// --sem-imagens.
    #[arg(long = "imagens", value_parser = BoolishValueParser::new())]
    pub imagens: Option<bool>,
}

impl McpArgs {
    /// As imagens vão para a IA? Só não com `--sem-imagens` ou `--imagens false`.
    pub fn images(&self) -> bool {
        !self.sem_imagens && self.imagens.unwrap_or(true)
    }
}

pub fn run(args: McpArgs) -> Result<()> {
    let images = args.images();
    serve_stdio(ServeOptions {
        album: args.album,
        allow: args.permitir,
        images,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::McpArgs;
    use crate::{Cli, Command};

    fn parse(args: &[&str]) -> McpArgs {
        let all = ["lamina", "mcp"].iter().chain(args);
        match Cli::try_parse_from(all).map(|c| c.command) {
            Ok(Command::Mcp(args)) => args,
            other => panic!("devia ler as opções do mcp: {other:?}"),
        }
    }

    /// Os argumentos que o Claude Desktop monta a partir do manifesto do pacote (IA-B24).
    #[test]
    fn opcoes_do_pacote_do_claude_desktop() {
        let args = parse(&[
            "--imagens",
            "false",
            "--permitir",
            "C:\\Fotos",
            "D:\\Álbuns",
        ]);
        assert_eq!(args.permitir.len(), 2);
        assert!(!args.images());
        assert!(parse(&["--imagens", "true", "--permitir", "C:\\Fotos"]).images());
    }

    #[test]
    fn album_antes_das_pastas_e_sem_imagens() {
        let args = parse(&[
            "album.lamina",
            "--permitir",
            "A",
            "--permitir",
            "B",
            "--sem-imagens",
        ]);
        assert_eq!(
            args.album.as_deref(),
            Some(std::path::Path::new("album.lamina"))
        );
        assert_eq!(args.permitir.len(), 2);
        assert!(!args.images());
        assert!(parse(&[]).images());
        let both = ["lamina", "mcp", "--sem-imagens", "--imagens", "true"];
        assert!(matches!(
            Cli::try_parse_from(both),
            Err(e) if e.kind() == clap::error::ErrorKind::ArgumentConflict
        ));
    }
}
