//! Desenho do álbum montado: uma lâmina por linha, com as miniaturas nos quadros (recorte
//! centralizado, como o `Crop` padrão) e uma legenda com papel, nota, família e motivo.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use lamina_core::{FrameContent, PageSide, PhotoId, Project, Rect};
use lamina_layout::{AutoBuild, SpreadReport};

use crate::importar::relative;
use crate::layouts::family_name;
use crate::plano::role_name;

/// Largura de cada lâmina desenhada, em pixels do SVG.
const SPREAD_PX: f64 = 640.0;
const MARGIN_PX: f64 = 24.0;
const CAPTION_PX: f64 = 40.0;

pub fn draw(built: &AutoBuild, thumbs: &HashMap<PhotoId, PathBuf>, folder: &Path) -> String {
    let project = &built.project;
    let trim = project.spec.trim_rect();
    let scale = SPREAD_PX / trim.width.get();
    let row = trim.height.get() * scale + CAPTION_PX + MARGIN_PX;
    let width = SPREAD_PX + 2.0 * MARGIN_PX;
    let height = MARGIN_PX + row * count(project.spreads.len());
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\"          font-family=\"sans-serif\" font-size=\"12\">
         <rect width=\"100%\" height=\"100%\" fill=\"#2a2d32\"/>
"
    );
    for (i, (spread, r)) in project.spreads.iter().zip(&built.report).enumerate() {
        let oy = MARGIN_PX + row * count(i);
        caption(&mut out, i, r, oy);
        let to_px = |rect: &Rect| {
            (
                MARGIN_PX + (rect.x - trim.x).get() * scale,
                oy + CAPTION_PX + (rect.y - trim.y).get() * scale,
                rect.width.get() * scale,
                rect.height.get() * scale,
            )
        };
        page_guides(&mut out, project, &to_px);
        for (n, f) in spread.frames.iter().enumerate() {
            let FrameContent::Photo { photo, .. } = &f.content else {
                continue;
            };
            let name = project
                .photo(*photo)
                .map(|p| relative(folder, &p.path))
                .unwrap_or_default();
            let shown = Shown {
                id: format!("q{i}_{n}"),
                px: to_px(&f.rect.to_absolute(&trim)),
                name,
                thumb: thumbs.get(photo).map(PathBuf::as_path),
            };
            frame(&mut out, &shown);
        }
    }
    out.push_str(
        "</svg>
",
    );
    out
}

/// Título e motivo da lâmina `i`, acima do desenho.
fn caption(out: &mut String, i: usize, r: &SpreadReport, oy: f64) {
    let title = format!(
        "Lâmina {} · momento {} · {} · {} fotos · nota {:.0}",
        i + 1,
        r.moment + 1,
        role_name(r.role),
        r.photos,
        r.score
    );
    let _ = writeln!(
        out,
        "<text x=\"{MARGIN_PX}\" y=\"{:.1}\" fill=\"#fff\" font-size=\"14\">{}</text>
         <text x=\"{MARGIN_PX}\" y=\"{:.1}\" fill=\"#9aa\" font-size=\"11\">{} — {}</text>",
        oy + 14.0,
        escape(&title),
        oy + 30.0,
        escape(&r.reason),
        escape(&family_name(&r.family))
    );
}

/// Um quadro já em pixels: identificador do recorte, posição, nome da foto e miniatura.
struct Shown<'a> {
    id: String,
    px: (f64, f64, f64, f64),
    name: String,
    thumb: Option<&'a Path>,
}

/// O quadro com a miniatura recortada no centro (como o `Crop` padrão) e o nome da foto.
fn frame(out: &mut String, s: &Shown) {
    let (x, y, w, h) = s.px;
    let id = &s.id;
    let _ = writeln!(
        out,
        "<clipPath id=\"{id}\"><rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\"/></clipPath>
         <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"#5b8def\"/>"
    );
    if let Some(thumb) = s.thumb {
        let _ = writeln!(
            out,
            "<image clip-path=\"url(#{id})\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\"              preserveAspectRatio=\"xMidYMid slice\" href=\"{}\"/>",
            escape(&file_url(thumb))
        );
    }
    let _ = writeln!(
        out,
        "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"#fff\" font-size=\"9\" stroke=\"#000\" stroke-width=\"0.3\">{}</text>",
        x + 3.0,
        y + h - 4.0,
        escape(&s.name)
    );
}

/// Fundo branco da lâmina, áreas úteis tracejadas e a lombada.
fn page_guides(
    out: &mut String,
    project: &Project,
    to_px: &impl Fn(&Rect) -> (f64, f64, f64, f64),
) {
    let (x, y, w, h) = to_px(&project.spec.trim_rect());
    let _ = writeln!(
        out,
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"#fff\"/>"
    );
    for side in [PageSide::Left, PageSide::Right] {
        let (x, y, w, h) = to_px(&project.spec.safe_rect(side));
        let _ = writeln!(
            out,
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"none\" stroke=\"#ccd\" stroke-dasharray=\"3 3\"/>"
        );
    }
    let gutter = x + w / 2.0;
    let _ = writeln!(
        out,
        "<line x1=\"{gutter:.1}\" y1=\"{y:.1}\" x2=\"{gutter:.1}\" y2=\"{:.1}\" stroke=\"#bbb\"/>",
        y + h
    );
}

/// Caminho local como endereço `file:///` (barras normais, espaços escapados).
fn file_url(path: &Path) -> String {
    let text = path
        .display()
        .to_string()
        .replace('\\', "/")
        .replace(' ', "%20");
    if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

/// Escapa o texto para dentro do SVG.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminho_vira_endereco_de_arquivo() {
        assert_eq!(
            file_url(Path::new(r"C:\Cache Lamina\a.jpg")),
            "file:///C:/Cache%20Lamina/a.jpg"
        );
        assert_eq!(file_url(Path::new("/tmp/a.jpg")), "file:///tmp/a.jpg");
    }

    #[test]
    fn desenha_uma_legenda_por_lamina_e_a_miniatura_de_cada_quadro() {
        use lamina_core::{AlbumSpec, Dpi, Mm};
        use lamina_layout::{BuildOptions, auto_build};

        let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap()
            .with_safe_margin(Mm(10.0))
            .unwrap();
        let mut project = Project::new("teste", spec);
        let a = project.add_photo("fotos/a.jpg", 3000, 2000);
        project.add_photo("fotos/b.jpg", 2000, 3000);
        let built = auto_build(&project, 1, &BuildOptions::default()).unwrap();
        let thumbs = HashMap::from([(a, PathBuf::from("/cache/a.jpg"))]);
        let svg = draw(&built, &thumbs, Path::new("fotos"));
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("Lâmina 1 · momento 1"));
        assert_eq!(svg.matches("<clipPath").count(), 2, "um recorte por quadro");
        assert_eq!(svg.matches("<image").count(), 1, "só a foto com miniatura");
        assert!(svg.contains("file:///cache/a.jpg") && svg.contains(">a.jpg<"));
    }

    #[test]
    fn texto_e_escapado() {
        assert_eq!(escape("Ana & <Bruno>"), "Ana &amp; &lt;Bruno&gt;");
    }
}
