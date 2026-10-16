//! Renderizar uma lâmina, codificar e gravar com segurança (spec EX, regras 1, 2 e 6).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use image::RgbaImage;
use image::imageops::crop_imm;
use lamina_core::{PageSide, Project};
use lamina_render::{PhotoSource, RenderTarget, render_spread};

use crate::jobs::{Output, SpreadJob};
use crate::{ExportError, jpeg};

/// O que toda lâmina precisa para ser gravada.
pub struct Ctx<'a> {
    pub project: &'a Project,
    pub folder: &'a Path,
    pub photos: &'a dyn PhotoSource,
    pub quality: u8,
}

/// Renderiza a lâmina uma vez e grava os arquivos dela.
pub fn spread(ctx: &Ctx, job: &SpreadJob) -> Result<Vec<PathBuf>, ExportError> {
    let dpi = ctx.project.spec.dpi().get();
    match &job.output {
        Output::Pages(pages) if pages.is_empty() => Ok(Vec::new()),
        Output::Whole(name) => {
            let image = render(ctx, job.spread)?;
            Ok(vec![save(ctx, name, image, dpi)?])
        }
        Output::Pages(pages) => {
            let image = render(ctx, job.spread)?;
            pages
                .iter()
                .map(|(side, name)| save(ctx, name, page_of(ctx.project, &image, *side), dpi))
                .collect()
        }
    }
}

fn render(ctx: &Ctx, spread: usize) -> Result<RgbaImage, ExportError> {
    Ok(render_spread(
        ctx.project,
        spread,
        RenderTarget::Print,
        ctx.photos,
    )?)
}

fn save(ctx: &Ctx, name: &str, image: RgbaImage, dpi: u32) -> Result<PathBuf, ExportError> {
    let bytes = jpeg::encode(image, ctx.quality, dpi).map_err(ExportError::Encode)?;
    let path = ctx.folder.join(name);
    write_atomic(ctx.folder, &path, &bytes)?;
    Ok(path)
}

/// A página com a própria sangria, recortada da imagem da lâmina. A largura sai da medida da
/// página (não da diferença de duas bordas arredondadas), então esquerda e direita têm sempre o
/// mesmo tamanho.
pub fn page_of(project: &Project, image: &RgbaImage, side: PageSide) -> RgbaImage {
    let canvas = project.spec.canvas_rect();
    let page = project.spec.page_canvas_rect(side);
    let scale = f64::from(image.width()) / canvas.width.get();
    let px = |mm: f64, max: u32| {
        // Preso entre 0 e o tamanho da imagem, então o cast não trunca.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (mm * scale).round().clamp(0.0, f64::from(max)) as u32;
        v
    };
    let (x, y) = (
        px(page.x.get(), image.width()),
        px(page.y.get(), image.height()),
    );
    let w = px(page.width.get(), image.width()).min(image.width().saturating_sub(x));
    let h = px(page.height.get(), image.height()).min(image.height().saturating_sub(y));
    crop_imm(image, x, y, w, h).to_image()
}

/// Grava num temporário de nome único na mesma pasta, força a ida ao disco e renomeia no fim.
/// Se algo falha, o temporário some sozinho.
fn write_atomic(folder: &Path, path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    let fail = |reason: String| ExportError::Write {
        path: path.display().to_string(),
        reason,
    };
    let mut tmp = tempfile::Builder::new()
        .prefix(".lamina-")
        .suffix(".tmp")
        .tempfile_in(folder)
        .map_err(|e| fail(e.to_string()))?;
    tmp.write_all(bytes).map_err(|e| fail(e.to_string()))?;
    tmp.as_file().sync_all().map_err(|e| fail(e.to_string()))?;
    tmp.persist(path).map_err(|e| fail(e.error.to_string()))?;
    Ok(())
}
