//! O painel contextual (spec AF, seção 4 e regras 10 a 12): sem seleção, o álbum; com uma foto,
//! a foto; Alt+4 esconde o painel e a janela lembra; a aba Detalhes só se abre sozinha no recorte,
//! no modo organizar e em "Ajustar a foto…" (AF-B20 a AF-B23).

use lamina_core::{FrameContent, PhotoAdjust};

use super::tl_common::{H, V, editor_at, spec, three_spreads};
use crate::editor::Details;
use crate::{Command, Shortcut, Strip, StripsStore};

fn rows(details: &Details) -> Vec<(String, String)> {
    match details {
        Details::Album(r) | Details::Photo(r) => r.clone(),
        _ => Vec::new(),
    }
}

fn value(details: &Details, title: &str) -> Option<String> {
    rows(details)
        .into_iter()
        .find(|(t, _)| t == title)
        .map(|(_, v)| v)
}

#[test]
fn af_b20_sem_selecao_mostra_o_album() {
    // 180 fotos: 110 horizontais e 70 verticais, em 12 lâminas; 3 com ajuste.
    let mut project = lamina_core::Project::new("teste", spec(30.0, 30.0));
    for i in 0..180 {
        let (w, h) = if i < 110 { H } else { V };
        project.add_photo(format!("f{i:03}.jpg"), w, h);
    }
    for _ in 0..12 {
        project.add_spread();
    }
    for p in project.photos.iter_mut().take(3) {
        p.adjust = PhotoAdjust {
            bw: true,
            ..PhotoAdjust::default()
        };
    }
    let editor = editor_at(project, 0);
    let d = editor.details();
    assert!(matches!(d, Details::Album(_)), "{d:?}");
    assert_eq!(value(&d, "Lâminas").unwrap(), "12 lâminas · 24 páginas");
    let fotos = value(&d, "Fotos").unwrap();
    assert!(
        fotos.starts_with("180 fotos · 110 horizontais, 70 verticais"),
        "{fotos}"
    );
    assert_eq!(value(&d, "Ajustes").unwrap(), "3 fotos com ajuste");
    assert_eq!(value(&d, "Perfil de saída").unwrap(), "sRGB");
    assert_eq!(value(&d, "Unidade").unwrap(), "cm");
}

#[test]
fn af_b21_foto_sem_camera_nao_mostra_as_linhas() {
    // As fotos do teste não têm arquivo: sem EXIF, as linhas da câmera não aparecem (a leitura
    // com velocidade e ISO é testada na lamina-media, `af_b21_informacoes_da_camera`).
    let mut editor = editor_at(three_spreads(), 1);
    let frame = {
        let s = editor.session().unwrap();
        s.project().spreads[1]
            .frames
            .iter()
            .position(|f| matches!(f.content, FrameContent::Photo { .. }))
            .unwrap()
    };
    editor.select_frame(Some(frame));
    let d = editor.details();
    assert!(matches!(d, Details::Photo(_)), "{d:?}");
    assert!(value(&d, "Arquivo").unwrap().ends_with(".jpg"));
    assert_eq!(value(&d, "Dimensões").unwrap(), "3000 × 2000 px");
    assert!(value(&d, "DPI efetivo").is_some());
    for missing in [
        "Velocidade",
        "ISO",
        "Abertura",
        "Distância focal",
        "Perfil de cor",
    ] {
        assert_eq!(
            value(&d, missing),
            None,
            "{missing} sem a informação não aparece"
        );
    }
}

#[test]
fn af_b22_esconder_o_painel_lembra() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config").join("janela.json");
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load(&file));
    assert!(editor.strips().is_open(Strip::Panel));
    let alt4 = Command::from_shortcut(Shortcut::alt('4')).unwrap();
    assert_eq!(alt4, Command::TogglePanel);
    assert!(editor.run(alt4));
    let mut next = editor_at(three_spreads(), 0);
    next.set_strips(StripsStore::load(&file));
    assert!(
        !next.strips().is_open(Strip::Panel),
        "continua escondido ao abrir de novo"
    );
}

#[test]
fn af_b23_a_aba_so_troca_no_recorte_e_no_modo() {
    let mut editor = editor_at(three_spreads(), 1);
    let frame = {
        let s = editor.session().unwrap();
        s.project().spreads[1]
            .frames
            .iter()
            .position(|f| matches!(f.content, FrameContent::Photo { .. }))
            .unwrap()
    };
    editor.select_frame(Some(frame));
    assert!(
        !editor.take_details_request(),
        "clicar numa foto não troca a aba"
    );
    assert!(editor.enter_crop(frame));
    assert!(
        editor.take_details_request(),
        "o recorte abre a aba Detalhes"
    );
    assert!(!editor.take_details_request(), "o pedido vale uma vez");
    editor.exit_crop();
    assert!(editor.run(Command::AdjustPhoto));
    assert!(editor.take_details_request(), "Ajustar a foto… abre a aba");
    assert!(editor.run(Command::Organize));
    assert!(editor.take_details_request(), "o modo organizar abre a aba");
}
