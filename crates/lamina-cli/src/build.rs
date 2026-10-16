//! `lamina build` (spec `comando-build`): importa, monta, grava o projeto e exporta, num comando.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use lamina_core::Project;
use lamina_editor::{import_project, suggestion, suggestion_text};
use lamina_export::{ExportOptions, output_names, preflight, project_file_name, save_project};
use lamina_layout::{AutoBuild, BuildOptions, auto_build};
use lamina_media::original_source;

use crate::auto::parse_reuse;
use crate::importar::cache_at;

use crate::relatorio::{export_with_progress, file_name, print_build_report, print_warnings};
use crate::spec::AlbumArgs;

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Pasta com as fotos (as subpastas também são lidas).
    pub pasta: PathBuf,
    /// Pasta onde gravar o projeto e os JPEGs de impressão.
    #[arg(long)]
    pub saida: PathBuf,
    /// Número de lâminas (padrão: sugestão de cerca de 4 fotos por lâmina).
    #[arg(long)]
    pub laminas: Option<usize>,
    /// Nome do projeto e do arquivo `.lamina` (padrão: nome da pasta de fotos).
    #[arg(long)]
    pub nome: Option<String>,
    #[command(flatten)]
    pub album: AlbumArgs,
    /// Mínimo de páginas do laboratório; abaixo dele, só um aviso.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub paginas_minimas: Option<u64>,
    /// Quanto repetir o mesmo desenho: baixa (mais variedade), media ou alta.
    #[arg(long, default_value = "media")]
    pub reutilizacao: String,
    /// Qualidade dos JPEGs (1 a 100; padrão 95, a de impressão).
    #[arg(long, default_value_t = lamina_export::DEFAULT_QUALITY)]
    pub qualidade: u8,
    /// Um arquivo por página em vez de um por lâmina.
    #[arg(long)]
    pub paginas: bool,
    /// Substituir o projeto e os JPEGs que já existem na saída.
    #[arg(long)]
    pub sobrescrever: bool,
    /// Pasta do catálogo e das miniaturas (padrão: cache do sistema).
    #[arg(long)]
    pub cache: Option<PathBuf>,
}

pub fn run(args: &BuildArgs) -> Result<()> {
    let project = imported(args)?;
    let spreads = spreads_for(args, &project);
    let options = BuildOptions {
        reuse: parse_reuse(&args.reutilizacao)?,
        ..BuildOptions::default()
    };
    let built = auto_build(&project, spreads, &options)?;
    print_build_report(&built);
    warn_min_pages(args, &built);
    let export = ExportOptions {
        quality: args.qualidade,
        range: None,
        pages: args.paginas,
        overwrite: args.sobrescrever,
    };
    // Tudo validado antes de gravar qualquer coisa (regra 5): nomes de saída e opções.
    let file = project_file_name(&built.project.name)?;
    let outputs = output_names(&built.project, &export)?;
    if !args.sobrescrever {
        refuse_existing(std::iter::once(&file).chain(&outputs), &args.saida)?;
    }
    std::fs::create_dir_all(&args.saida)
        .with_context(|| format!("Não foi possível criar a pasta {}.", args.saida.display()))?;
    let saved = save_project(&built.project, &args.saida, &file)?;
    let all: Vec<usize> = (0..spreads).collect();
    print_warnings(&preflight(&built.project, &all));
    export_with_progress(&built.project, &args.saida, &original_source(), &export)?;
    println!("Projeto: {}", saved.display());
    Ok(())
}

/// Importa a pasta e monta o projeto, com o nome pedido ou o da pasta: o mesmo caminho do
/// "Novo álbum" da janela (spec JP, regra 3).
fn imported(args: &BuildArgs) -> Result<Project> {
    let cache = cache_at(args.cache.clone());
    let spec = args.album.to_spec()?;
    let name = args.nome.as_deref();
    let imported = import_project(&args.pasta, name, spec, &cache, false, &|_| {})?;
    for (path, reason) in &imported.failures {
        println!("Não foi possível importar {}: {reason}.", file_name(path));
    }
    Ok(imported.project)
}

/// As lâminas pedidas ou, sem `--laminas`, a sugestão (mostrada na tela).
fn spreads_for(args: &BuildArgs, project: &Project) -> usize {
    args.laminas.unwrap_or_else(|| {
        let n = suggestion(project);
        println!("{}", suggestion_text(n));
        n
    })
}

/// Aviso (sem bloquear) quando o álbum tem menos páginas que o mínimo pedido (regra 3).
fn warn_min_pages(args: &BuildArgs, built: &AutoBuild) {
    let Some(min) = args.paginas_minimas else {
        return;
    };
    let pages = built.project.spec.edges().page_count(built.report.len());
    if (pages as u64) < min {
        println!("O álbum tem {pages} páginas; o mínimo pedido é {min}.");
    }
}

/// Nada é gravado se o projeto ou algum JPEG já existe na saída (regra 5).
fn refuse_existing<'a>(names: impl Iterator<Item = &'a String>, folder: &Path) -> Result<()> {
    for name in names {
        if folder.join(name).exists() {
            bail!(
                "Já existe {name} em {}; use --sobrescrever para substituir.",
                folder.display()
            );
        }
    }
    Ok(())
}
