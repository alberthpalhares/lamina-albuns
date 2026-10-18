//! Texto nas lâminas (spec TX): fontes embutidas, linhas calculadas uma vez e desenho em
//! qualquer escala.

mod draw;
mod fonts;
mod layout;

use image::RgbaImage;
use lamina_core::TextBox;
use skrifa::{FontRef, MetadataProvider};
use tiny_skia::Pixmap;

pub use fonts::{FAMILIES, Family, family, known_families};
pub use layout::text_lines;

use fonts::family_or_default;
use layout::{lay_out, pt};

/// O que a pré-impressão precisa saber de um texto (spec TX, seção 4).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextCheck {
    /// O bloco passa da altura do quadro.
    pub overflow: bool,
    /// Caracteres que a fonte não tem, na ordem em que aparecem, sem repetir.
    pub missing: Vec<char>,
    /// A fonte do projeto não existe nesta versão (o texto sai na padrão).
    pub unknown_font: bool,
}

/// Confere o texto num quadro de `size_mm` (largura, altura).
pub fn check_text(text: &TextBox, size_mm: (f64, f64)) -> TextCheck {
    if text.is_blank() {
        return TextCheck::default();
    }
    let drawn = family_or_default(&text.font);
    let mut missing = Vec::new();
    if let Ok(face) = FontRef::new(drawn.face(text.italic && drawn.italic)) {
        let map = face.charmap();
        for c in text.text.chars().filter(|c| !c.is_whitespace()) {
            if map.map(c).is_none() && !missing.contains(&c) {
                missing.push(c);
            }
        }
    }
    TextCheck {
        overflow: lay_out(text, size_mm).overflow,
        missing,
        unknown_font: family(&text.font).is_none(),
    }
}

/// O texto desenhado numa imagem transparente de `px` pixels, para um quadro de `size_mm`. As
/// cores saem pré-multiplicadas pelo alfa. `None` para texto vazio ou tamanho zero.
pub(crate) fn text_pixmap(text: &TextBox, size_mm: (f64, f64), px: (u32, u32)) -> Option<Pixmap> {
    if text.is_blank() || size_mm.0 <= 0.0 {
        return None;
    }
    let mut pixmap = Pixmap::new(px.0, px.1)?;
    let laid = lay_out(text, size_mm);
    // Pixels por ponto, pela largura (a altura tem a mesma escala, a menos do arredondamento).
    #[allow(clippy::cast_possible_truncation)]
    let k = (f64::from(px.0) / pt(size_mm.0)) as f32;
    draw::draw(&mut pixmap, &laid, text.color, k);
    Some(pixmap)
}

/// Como [`text_pixmap`], numa `RgbaImage` (cores pré-multiplicadas), para a janela.
pub fn text_image(text: &TextBox, size_mm: (f64, f64), px: (u32, u32)) -> Option<RgbaImage> {
    let pixmap = text_pixmap(text, size_mm, px)?;
    RgbaImage::from_raw(px.0, px.1, pixmap.take())
}

/// Desenha os textos da lâmina por cima das fotos, na ordem dos quadros (spec TX, regra 5).
pub(crate) fn draw_texts(
    pixmap: &mut Pixmap,
    project: &lamina_core::Project,
    spread: &lamina_core::Spread,
    scale: crate::place::Scale,
) {
    let size = (pixmap.width(), pixmap.height());
    let trim = project.spec.trim_rect();
    for frame in &spread.frames {
        let lamina_core::FrameContent::Text(text) = &frame.content else {
            continue;
        };
        let rect = frame.rect.to_absolute(&trim);
        let Some(at) = crate::place::pixels(rect, scale, size) else {
            continue;
        };
        let size_mm = (rect.width.get(), rect.height.get());
        if let Some(t) = text_pixmap(text, size_mm, (at.width(), at.height())) {
            // As posições cabem em i32: a tela tem no máximo alguns milhares de pixels.
            #[allow(clippy::cast_possible_wrap)]
            let (x, y) = (at.x0 as i32, at.y0 as i32);
            pixmap.draw_pixmap(
                x,
                y,
                t.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }
}
