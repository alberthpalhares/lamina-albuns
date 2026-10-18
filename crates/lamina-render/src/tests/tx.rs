//! Cenários da spec `texto-e-quadros` (TX) que moram no renderizador: B2, B4, B13 (só as fontes
//! embutidas), B15 e B16. Os outros ficam no editor e na exportação.

use std::time::{Duration, Instant};

use image::RgbaImage;
use lamina_core::testing::{TRIES, budget};
use lamina_core::{
    AlbumSpec, Dpi, FrameContent, Mm, Project, RelRect, Rgb, TextAlign, TextBox, TextVAlign,
};

use crate::{
    FAMILIES, RenderTarget, check_text, known_families, render_spread, text_image, text_lines,
};

fn text(s: &str, font: &str, size_pt: f64) -> TextBox {
    TextBox {
        text: s.to_owned(),
        font: font.to_owned(),
        size_pt,
        ..TextBox::new(Rgb::BLACK)
    }
}

/// A caixa das partes pintadas, em frações da imagem: (esquerda, topo, direita, base).
fn ink(img: &RgbaImage) -> (f64, f64, f64, f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 127 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
    }
    let (w, h) = (f64::from(img.width()), f64::from(img.height()));
    (
        f64::from(x0) / w,
        f64::from(y0) / h,
        f64::from(x1) / w,
        f64::from(y1) / h,
    )
}

const FRASE: &str = "Num sábado de sol, entre amigos e família, Ana e Pedro disseram sim \
    debaixo da velha figueira do sítio, e a festa seguiu até a última música da noite, \
    com bolo, dança e muita conversa boa no quintal iluminado.";

#[test]
fn tx_b2_mesmas_linhas_na_tela_e_na_impressao() {
    let t = TextBox {
        align: TextAlign::Left,
        valign: TextVAlign::Top,
        ..text(FRASE, "EB Garamond", 14.0)
    };
    let size_mm = (100.0, 40.0);
    let lines = text_lines(&t, size_mm);
    assert!(lines.len() >= 3, "{lines:?}");
    // Tela: 400 px de largura; impressão: 300 DPI (1181 px).
    let screen = text_image(&t, size_mm, (400, 160)).unwrap();
    let print = text_image(&t, size_mm, (1181, 472)).unwrap();
    let (a, b) = (ink(&screen), ink(&print));
    let px = 1.0 / 400.0;
    for (s, p) in [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)] {
        assert!((s - p).abs() < 1.5 * px, "tela {a:?} × impressão {b:?}");
    }
}

#[test]
fn tx_b4_texto_que_nao_cabe() {
    let t = text("Ana\nPedro\nSempre", "Inter", 24.0);
    assert!(check_text(&t, (40.0, 10.0)).overflow);
    assert!(!check_text(&t, (40.0, 40.0)).overflow);
}

#[test]
fn tx_b13_so_as_fontes_embutidas() {
    let mut names: Vec<String> = FAMILIES.iter().map(|f| f.name.to_owned()).collect();
    names.sort();
    assert_eq!(known_families(), names);
}

#[test]
fn tx_b15_caractere_que_a_fonte_nao_tem() {
    let t = text("Ana 🙂 Pedro 🙂", "Inter", 24.0);
    let check = check_text(&t, (100.0, 30.0));
    assert_eq!(check.missing, ['🙂']);
    assert!(!check.unknown_font);
    assert!(check_text(&text("Lobster", "Lobster", 12.0), (50.0, 20.0)).unknown_font);
}

fn spread_with_texts(n: usize) -> Project {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut project = Project::new("teste", spec);
    let spread = project.add_spread();
    for i in 0..n {
        #[allow(clippy::cast_precision_loss)]
        let y = 0.05 + 0.18 * i as f64;
        let rect = RelRect::new(0.1, y, 0.35, 0.15).unwrap();
        let content = FrameContent::Text(text(&FRASE[..120], FAMILIES[i % 6].name, 18.0));
        project.add_frame(spread, rect, content).unwrap();
    }
    project
}

#[test]
fn tx_b16_texto_rapido() {
    let t = text(&FRASE[..200], "Inter", 16.0);
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let started = Instant::now();
        let _ = text_image(&t, (120.0, 60.0), (1000, 500));
        best = best.min(started.elapsed());
        if best < budget(30) {
            break;
        }
    }
    assert!(best < budget(30), "desenhar o texto levou {best:?}");
    // Exportar uma lâmina com cinco textos custa menos de 300 ms a mais.
    let none = |_: &lamina_core::Photo, _: (u32, u32)| Err("sem fotos".to_owned());
    let mut extra = Duration::MAX;
    for _ in 0..TRIES {
        let (empty, full) = (spread_with_texts(0), spread_with_texts(5));
        let started = Instant::now();
        render_spread(&empty, 0, RenderTarget::Print, &none).unwrap();
        let base = started.elapsed();
        let started = Instant::now();
        render_spread(&full, 0, RenderTarget::Print, &none).unwrap();
        extra = extra.min(started.elapsed().saturating_sub(base));
        if extra < budget(300) {
            break;
        }
    }
    assert!(extra < budget(300), "os textos custaram {extra:?} a mais");
}

/// As faixas de linhas com tinta, em frações da altura: (começo, fim) de cada linha de texto.
fn bands(img: &RgbaImage) -> Vec<(f64, f64)> {
    let h = f64::from(img.height());
    let inked = |y: u32| (0..img.width()).any(|x| img.get_pixel(x, y)[3] > 127);
    let mut out = Vec::new();
    let mut start = None;
    for y in 0..img.height() {
        match (inked(y), start) {
            (true, None) => start = Some(y),
            (false, Some(s)) => {
                out.push((f64::from(s) / h, f64::from(y) / h));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((f64::from(s) / h, 1.0));
    }
    out
}

#[test]
fn tx_b2_cada_linha_no_mesmo_lugar_nas_duas_escalas() {
    let t = TextBox {
        align: TextAlign::Left,
        valign: TextVAlign::Top,
        line_spacing: 1.6,
        ..text(FRASE, "Inter", 14.0)
    };
    let size_mm = (100.0, 60.0);
    let screen = bands(&text_image(&t, size_mm, (400, 240)).unwrap());
    let print = bands(&text_image(&t, size_mm, (1181, 709)).unwrap());
    assert_eq!(screen.len(), print.len(), "{screen:?} × {print:?}");
    assert_eq!(screen.len(), text_lines(&t, size_mm).len());
    let px = 1.5 / 240.0;
    for (s, p) in screen.iter().zip(&print) {
        assert!(
            (s.0 - p.0).abs() < px && (s.1 - p.1).abs() < px,
            "{s:?} × {p:?}"
        );
    }
}

/// Uma lâmina 30 × 30 sem foto, só com o texto no quadro `rect`, em prévia de 1200 pixels.
fn render_text(rect: [f64; 4], t: TextBox) -> (RgbaImage, lamina_core::Rect, f64) {
    let mut project = spread_with_texts(0);
    let id = project.spreads[0].id;
    let rel = RelRect::new(rect[0], rect[1], rect[2], rect[3]).unwrap();
    project.add_frame(id, rel, FrameContent::Text(t)).unwrap();
    let none = |_: &lamina_core::Photo, _: (u32, u32)| Err("sem fotos".to_owned());
    let img = render_spread(&project, 0, RenderTarget::Preview { max_side: 1200 }, &none).unwrap();
    let r = rel.to_absolute(&project.spec.trim_rect());
    let k = f64::from(img.width()) / project.spec.canvas_rect().width.get();
    (img, r, k)
}

// Medidas de tela: alguns milhares de pixels, cabem em u32.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn px(v: f64) -> u32 {
    v.round().max(0.0) as u32
}

#[test]
fn tx_b4_a_exportacao_corta_o_texto_na_borda_do_quadro() {
    let t = text("Ana\nPedro\nSempre\nJuntos", "Inter", 40.0);
    let (img, r, k) = render_text([0.3, 0.3, 0.2, 0.05], t);
    let below = px((r.y.get() + r.height.get()) * k) + 2;
    let ink = (px(r.x.get() * k)..px((r.x.get() + r.width.get()) * k))
        .flat_map(|x| (below..below + 40).map(move |y| (x, y)))
        .any(|(x, y)| img.get_pixel(x, y)[0] < 200);
    assert!(!ink, "o texto vazou abaixo do quadro");
}

#[test]
fn texto_encostado_no_corte_nao_pinta_a_sangria() {
    let t = TextBox {
        align: TextAlign::Left,
        ..text("Ana & Pedro", "Inter", 30.0)
    };
    let (img, r, k) = render_text([0.0, 0.4, 0.2, 0.1], t);
    let left = px(r.x.get() * k);
    let ink = (0..left.saturating_sub(1))
        .flat_map(|x| {
            (px(r.y.get() * k)..px((r.y.get() + r.height.get()) * k)).map(move |y| (x, y))
        })
        .any(|(x, y)| img.get_pixel(x, y)[0] < 200);
    assert!(left > 2, "precisa de sangria no teste");
    assert!(!ink, "o texto entrou na sangria");
}
