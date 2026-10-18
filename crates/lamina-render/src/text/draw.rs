//! Desenhar as linhas prontas numa imagem do tamanho do quadro: o contorno de cada letra
//! (`skrifa`) vira um caminho do `tiny-skia`, na escala pedida. A mesma função serve à tela e à
//! impressão (ADR 0006).

use lamina_core::Rgb;
use parley::PositionedLayoutItem;
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

use super::layout::Laid;

/// Junta os comandos do contorno num caminho do `tiny-skia`, já em pixels.
struct Pen {
    path: PathBuilder,
    /// Pixels por ponto.
    k: f32,
    /// Origem da letra, em pixels (o y do contorno cresce para cima).
    at: (f32, f32),
}

impl Pen {
    fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.at.0 + x * self.k, self.at.1 - y * self.k)
    }
}

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.p(x, y);
        self.path.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.p(x, y);
        self.path.line_to(x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let ((cx, cy), (x, y)) = (self.p(cx, cy), self.p(x, y));
        self.path.quad_to(cx, cy, x, y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let ((ax, ay), (bx, by), (x, y)) = (self.p(cx0, cy0), self.p(cx1, cy1), self.p(x, y));
        self.path.cubic_to(ax, ay, bx, by, x, y);
    }
    fn close(&mut self) {
        self.path.close();
    }
}

/// Desenha `laid` em `pixmap` (do tamanho do quadro), com `k` pixels por ponto.
pub(crate) fn draw(pixmap: &mut Pixmap, laid: &Laid, color: Rgb, k: f32) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, 255);
    paint.anti_alias = true;
    for line in laid.layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(run) = item else {
                continue;
            };
            let font = run.run().font();
            let Ok(face) = FontRef::from_index(font.data.as_ref(), font.index) else {
                continue;
            };
            let coords: Vec<NormalizedCoord> = run
                .run()
                .normalized_coords()
                .iter()
                .map(|c| NormalizedCoord::from_bits(*c))
                .collect();
            let outlines = face.outline_glyphs();
            let size = Size::new(run.run().font_size());
            let mut pen = Pen {
                path: PathBuilder::new(),
                k,
                at: (0.0, 0.0),
            };
            for g in run.positioned_glyphs() {
                let Some(glyph) = outlines.get(GlyphId::new(g.id)) else {
                    continue;
                };
                pen.at = (g.x * k, (laid.top + g.y) * k);
                let settings = DrawSettings::unhinted(size, LocationRef::new(&coords));
                // Uma letra que não desenha (contorno quebrado) só fica de fora.
                let _ = glyph.draw(settings, &mut pen);
            }
            let path = std::mem::replace(&mut pen.path, PathBuilder::new());
            if let Some(path) = path.finish() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
    }
}
