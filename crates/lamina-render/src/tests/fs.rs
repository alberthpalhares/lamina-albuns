//! Cenários da spec `fontes-do-computador` (FS) que moram no renderizador: B1, B3 a B6, B10, B13
//! a B15 e B17. Os outros ficam no editor e na exportação.

use std::time::{Duration, Instant};

use image::RgbaImage;
use lamina_core::testing::{TRIES, budget};
use lamina_core::{AlbumSpec, Dpi, FrameContent, Mm, Project, RelRect, Rgb, TextBox};

use crate::testing::{FONT_A, FONT_B, TestFonts};
use crate::text::source::{FontSource, SystemSource};
use crate::{
    FAMILIES, FontStyles, RenderTarget, check_text, font_preview, font_styles, installed_name,
    installed_names, render_spread, text_image,
};

fn text(s: &str, font: &str) -> TextBox {
    TextBox {
        text: s.to_owned(),
        font: font.to_owned(),
        size_pt: 24.0,
        ..TextBox::new(Rgb::BLACK)
    }
}

fn image(font: &str) -> RgbaImage {
    text_image(&text("Ana & Pedro", font), (80.0, 20.0), (400, 100)).unwrap()
}

/// Uma lâmina com um texto em cada fonte de `fonts`.
fn album(fonts: &[&str]) -> Project {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(100).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut project = Project::new("teste", spec);
    let spread = project.add_spread();
    for (i, font) in fonts.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let y = 0.02 + 0.13 * i as f64;
        let rect = RelRect::new(0.1, y, 0.6, 0.12).unwrap();
        let content = FrameContent::Text(text("Ana & Pedro", font));
        project.add_frame(spread, rect, content).unwrap();
    }
    project
}

fn render(project: &Project) -> RgbaImage {
    let none = |_: &lamina_core::Photo, _: (u32, u32)| Err("sem fotos".to_owned());
    render_spread(project, 0, RenderTarget::Print, &none).unwrap()
}

#[test]
fn fs_b1_os_nomes_do_computador_vem_em_ordem_e_com_previa() {
    let _fonts = TestFonts::standard();
    assert_eq!(*installed_names(), [FONT_A, FONT_B]);
    assert!(font_preview(FONT_A, (240, 28)).is_some());
    assert!(font_preview("Inter", (240, 28)).is_some());
    assert!(font_preview("Lobster", (240, 28)).is_none());
}

#[test]
fn fs_b3_a_fonte_do_computador_muda_o_desenho() {
    let _fonts = TestFonts::standard();
    assert_ne!(image(FONT_A), image("Inter"));
    assert!(!check_text(&text("Ana", FONT_A), (80.0, 20.0)).unknown_font);
}

#[test]
fn fs_b4_negrito_e_italico_seguem_os_arquivos_da_familia() {
    let _fonts = TestFonts::standard();
    let none = FontStyles {
        bold: false,
        italic: false,
    };
    let both = FontStyles {
        bold: true,
        italic: true,
    };
    assert_eq!(font_styles(FONT_A), Some(none));
    assert_eq!(font_styles(FONT_B), Some(both));
    assert_eq!(font_styles("Lobster"), None);
    // O negrito da "Fonte B" desenha mesmo (a fonte variável chega a 700).
    let bold = TextBox {
        bold: true,
        ..text("Ana & Pedro", FONT_B)
    };
    let drawn = text_image(&bold, (80.0, 20.0), (400, 100)).unwrap();
    assert_ne!(drawn, image(FONT_B));
}

#[test]
fn fs_b5_mesmo_nome_vale_a_embutida() {
    let plain = {
        let _fonts = TestFonts::none();
        image("Inter")
    };
    let _fonts = TestFonts::with_inter();
    assert!(!installed_names().iter().any(|n| n == "Inter"));
    assert_eq!(image("Inter"), plain);
}

#[test]
fn fs_b6_fonte_que_nao_esta_neste_computador_sai_em_inter() {
    let _fonts = TestFonts::standard();
    assert!(check_text(&text("Ana", "Lobster"), (80.0, 20.0)).unknown_font);
    assert_eq!(image("Lobster"), image("Inter"));
}

#[test]
fn fs_b10_mesma_maquina_mesmo_desenho() {
    let _fonts = TestFonts::standard();
    let mut fonts: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
    fonts.push(FONT_A);
    let project = album(&fonts);
    assert_eq!(render(&project), render(&project));
}

#[test]
fn fs_b13_as_embutidas_nao_leem_as_fontes_do_computador() {
    let fonts = TestFonts::standard();
    let names: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
    render(&album(&names));
    for name in &names {
        assert!(font_styles(name).is_some());
        let _ = check_text(&text("Ana", name), (80.0, 20.0));
    }
    assert_eq!(fonts.reads().names, 0);
    assert_eq!(fonts.reads().files, 0);
}

#[test]
fn fs_b14_os_nomes_sao_lidos_uma_vez() {
    let fonts = TestFonts::standard();
    installed_names();
    installed_names();
    assert_eq!(installed_name("fonte b").as_deref(), Some(FONT_B));
    let _ = image(FONT_A);
    assert_eq!(fonts.reads().names, 1);
}

#[test]
fn fs_b15_so_o_arquivo_da_fonte_usada_e_aberto_e_as_linhas_dividem() {
    let fonts = TestFonts::many(2000);
    assert_eq!(installed_names().len(), 2002);
    let project = album(&[FONT_A]);
    let images: Vec<RgbaImage> = std::thread::scope(|s| {
        let runs: Vec<_> = (0..4).map(|_| s.spawn(|| render(&project))).collect();
        runs.into_iter().map(|r| r.join().unwrap()).collect()
    });
    assert!(images.windows(2).all(|w| w[0] == w[1]));
    // Um arquivo só: a "Fonte A" tem uma face, aberta uma vez para as 4 linhas.
    assert_eq!(fonts.reads().files, 1);
}

#[test]
fn fs_b17_ler_os_nomes_deste_computador_e_rapido() {
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let mut source = SystemSource::default();
        let started = Instant::now();
        let names = source.names();
        best = best.min(started.elapsed());
        drop(names);
        if best < budget(300) {
            break;
        }
    }
    assert!(best < budget(300), "ler os nomes das fontes levou {best:?}");
}
