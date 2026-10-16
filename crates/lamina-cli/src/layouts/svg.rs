//! Desenho das sugestões num SVG: uma seção por lista, com número, nota e família de cada opção.

use std::fmt::Write as _;

use lamina_core::{ActivePages, AlbumSpec, PageSide, Rect};
use lamina_layout::LayoutCandidate;

use super::describe;

/// Largura de cada lâmina desenhada, em pixels do SVG.
const SPREAD_PX: f64 = 380.0;
const COLUMNS: usize = 3;
const MARGIN_PX: f64 = 24.0;
const CAPTION_PX: f64 = 34.0;
const TITLE_PX: f64 = 40.0;

/// Uma lista com seu título.
pub type Section<'a> = (&'a str, &'a [LayoutCandidate]);

pub fn draw(
    spec: &AlbumSpec,
    active: ActivePages,
    aspects: &[f64],
    sections: &[Section],
) -> String {
    let trim = spec.trim_rect();
    let cell_h = trim.height.get() * SPREAD_PX / trim.width.get() + CAPTION_PX + MARGIN_PX;
    let width = MARGIN_PX + (SPREAD_PX + MARGIN_PX) * count(COLUMNS);
    let height: f64 = MARGIN_PX
        + sections
            .iter()
            .map(|(_, list)| TITLE_PX + cell_h * count(list.len().div_ceil(COLUMNS).max(1)))
            .sum::<f64>();
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" \
         font-family=\"sans-serif\" font-size=\"12\">\n<rect width=\"100%\" height=\"100%\" fill=\"#2a2d32\"/>\n"
    );
    let mut top = MARGIN_PX;
    for (title, list) in sections {
        let _ = writeln!(
            out,
            "<text x=\"{MARGIN_PX}\" y=\"{:.1}\" fill=\"#fff\" font-size=\"18\">{title} ({})</text>",
            top + 20.0,
            list.len()
        );
        top += TITLE_PX;
        for (i, candidate) in list.iter().enumerate() {
            let ox = MARGIN_PX + (SPREAD_PX + MARGIN_PX) * count(i % COLUMNS);
            let oy = top + cell_h * count(i / COLUMNS);
            spread(&mut out, (spec, active, aspects), candidate, (ox, oy));
            let caption = format!("#{} · nota {:.0}", i + 1, candidate.score.total);
            let y = oy + trim.height.get() * SPREAD_PX / trim.width.get();
            let _ = writeln!(
                out,
                "<text x=\"{ox:.1}\" y=\"{:.1}\" fill=\"#ddd\">{caption}</text>\n\
                 <text x=\"{ox:.1}\" y=\"{:.1}\" fill=\"#9aa\" font-size=\"10\">{}</text>",
                y + 15.0,
                y + 28.0,
                describe(candidate)
            );
        }
        top += cell_h * count(list.len().div_ceil(COLUMNS).max(1));
    }
    out.push_str("</svg>\n");
    out
}

/// Uma lâmina: corte, áreas úteis, lombada e os quadros numerados.
fn spread(
    out: &mut String,
    (spec, active, aspects): (&AlbumSpec, ActivePages, &[f64]),
    candidate: &LayoutCandidate,
    (ox, oy): (f64, f64),
) {
    let trim = spec.trim_rect();
    let scale = SPREAD_PX / trim.width.get();
    let to_px = |r: &Rect| {
        (
            ox + (r.x - trim.x).get() * scale,
            oy + (r.y - trim.y).get() * scale,
            r.width.get() * scale,
            r.height.get() * scale,
        )
    };
    let (x, y, w, h) = to_px(&trim);
    let _ = writeln!(
        out,
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"#fff\"/>"
    );
    for (on, page) in [
        (active.left, PageSide::Left),
        (active.right, PageSide::Right),
    ] {
        let (x, y, w, h) = to_px(&spec.safe_rect(page));
        let fill = if on { "none" } else { "#e6e6e6" };
        let _ = writeln!(
            out,
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"{fill}\" stroke=\"#9ab\" stroke-dasharray=\"3 3\"/>"
        );
    }
    let gutter = ox + SPREAD_PX / 2.0;
    let _ = writeln!(
        out,
        "<line x1=\"{gutter:.1}\" y1=\"{y:.1}\" x2=\"{gutter:.1}\" y2=\"{:.1}\" stroke=\"#ccc\"/>",
        y + h
    );
    for (n, (frame, aspect)) in candidate.frames.iter().zip(aspects).enumerate() {
        let (fx, fy, fw, fh) = to_px(&frame.to_absolute(&trim));
        let (color, label) = match aspect {
            a if (a - 1.0).abs() < 0.02 => ("#7fb069", "Q"),
            a if *a > 1.0 => ("#5b8def", "H"),
            _ => ("#e8a838", "V"),
        };
        let _ = writeln!(
            out,
            "<rect x=\"{fx:.1}\" y=\"{fy:.1}\" width=\"{fw:.1}\" height=\"{fh:.1}\" fill=\"{color}\" fill-opacity=\"0.85\" stroke=\"#fff\" stroke-width=\"0.5\"/>\n\
             <text x=\"{:.1}\" y=\"{:.1}\" fill=\"#fff\" text-anchor=\"middle\">{}{}</text>",
            fx + fw / 2.0,
            fy + fh / 2.0 + 4.0,
            n + 1,
            label
        );
    }
}

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use lamina_core::{Dpi, Mm};
    use lamina_layout::{LayoutOptions, layout_options};

    use super::*;

    #[test]
    fn desenha_uma_secao_por_lista_e_um_quadro_por_foto() {
        let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap()
            .with_safe_margin(Mm(10.0))
            .unwrap();
        let both = ActivePages {
            left: true,
            right: true,
        };
        let aspects = [1.5, 2.0 / 3.0];
        let lists = layout_options(&spec, both, &aspects, &LayoutOptions::default()).unwrap();
        let first = &lists.in_order[..1];
        let svg = draw(
            &spec,
            both,
            &aspects,
            &[("Na ordem", first), ("Reordenadas", &[])],
        );
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("Na ordem (1)") && svg.contains("Reordenadas (0)"));
        // Um quadro por foto, com o número e a orientação.
        assert!(svg.contains(">1H<") && svg.contains(">2V<"));
    }
}
