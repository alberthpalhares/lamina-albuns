//! As linhas de um quadro de texto (spec TX, regra 1). Tudo é calculado em pontos, sem depender
//! da resolução: a tela e a impressão usam as mesmas linhas e só mudam a escala do desenho.

use std::borrow::Cow;

use lamina_core::{MM_PER_INCH, TextAlign, TextBox};
use parley::{
    Alignment, AlignmentOptions, FontFamily, FontStyle, FontWeight, Layout, LineHeight,
    OverflowWrap, StyleProperty,
};

use super::fonts::{drawn_or_default, with_context};

/// Pontos tipográficos por polegada.
const PT_PER_INCH: f64 = 72.0;

/// Milímetros → pontos.
pub(crate) fn pt(mm: f64) -> f64 {
    mm * PT_PER_INCH / MM_PER_INCH
}

/// O texto arrumado num quadro de `size_mm`: as linhas e onde o bloco começa, em pontos.
pub(crate) struct Laid {
    pub layout: Layout<[u8; 4]>,
    /// Distância do topo do quadro ao topo da 1ª linha (alinhamento vertical).
    pub top: f32,
    /// O bloco passa da altura do quadro (regra 3).
    pub overflow: bool,
}

// Medidas de quadro em pontos cabem com folga num f32.
#[allow(clippy::cast_possible_truncation)]
fn f32_of(v: f64) -> f32 {
    v as f32
}

/// Monta as linhas de `text` num quadro de `size_mm` (largura, altura).
pub(crate) fn lay_out(text: &TextBox, size_mm: (f64, f64)) -> Laid {
    let family = drawn_or_default(&text.font);
    let (width, height) = (f32_of(pt(size_mm.0)), f32_of(pt(size_mm.1)));
    let mut layout = with_context(&family, |fonts, scratch| {
        // `false`: sem arredondar as posições para a grade, que aqui é de pontos (regra 1).
        let mut b = scratch.ranged_builder(fonts, &text.text, 1.0, false);
        b.push_default(StyleProperty::FontFamily(FontFamily::named(family.name())));
        b.push_default(StyleProperty::FontSize(f32_of(text.size_pt)));
        b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            f32_of(text.line_spacing),
        )));
        // Palavra maior que o quadro quebra no meio, em vez de vazar pela lateral.
        b.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
        if text.bold && family.bold() {
            b.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        }
        if text.italic && family.italic() {
            b.push_default(StyleProperty::FontStyle(FontStyle::Italic));
        }
        b.build(&text.text)
    });
    layout.break_all_lines(Some(width));
    let alignment = match text.align {
        TextAlign::Left => Alignment::Left,
        TextAlign::Center => Alignment::Center,
        TextAlign::Right => Alignment::Right,
    };
    layout.align(alignment, AlignmentOptions::default());
    // O alinhamento vale dentro da largura do quadro, não só da linha mais larga.
    let spare = (height - layout.height()).max(0.0);
    let top = match text.valign {
        lamina_core::TextVAlign::Top => 0.0,
        lamina_core::TextVAlign::Middle => spare / 2.0,
        lamina_core::TextVAlign::Bottom => spare,
    };
    let overflow = layout.height() > height + 0.01;
    Laid {
        layout,
        top,
        overflow,
    }
}

/// O texto de cada linha, sem os espaços do fim (para conferir as quebras, TX-B2).
pub fn text_lines(text: &TextBox, size_mm: (f64, f64)) -> Vec<String> {
    let laid = lay_out(text, size_mm);
    laid.layout
        .lines()
        .map(|line| {
            let range = line.text_range();
            Cow::Borrowed(text.text.get(range).unwrap_or_default())
                .trim_end()
                .to_owned()
        })
        .collect()
}
