//! Cenários da spec `paineis` (PN): as faixas de baixo com título e recolhíveis.

use super::tl_common::{H, built, editor_at, spec, three_spreads};
use crate::photos::used_photos;
use crate::{
    Command, Editor, Menu, STRIPS_UNREADABLE, Shortcut, Strip, Strips, StripsStore, strip_titles,
};

fn open(editor: &Editor) -> [bool; 3] {
    let s = editor.strips();
    [
        s.is_open(Strip::Spreads),
        s.is_open(Strip::Timeline),
        s.is_open(Strip::Photos),
    ]
}

#[test]
fn pn_b1_recolher_e_abrir_de_novo() {
    let mut editor = editor_at(three_spreads(), 0);
    assert_eq!(open(&editor), [true, true, true]);
    assert!(editor.run(Command::ToggleTimeline));
    assert_eq!(open(&editor), [true, false, true]);
    assert!(editor.run(Command::ToggleTimeline));
    assert_eq!(open(&editor), [true, true, true]);
}

#[test]
fn pn_b2_mesmo_comando_pelo_menu_e_pelo_atalho() {
    let expected = [
        ('1', Command::ToggleSpreads, "Faixa de lâminas"),
        ('2', Command::ToggleTimeline, "Linha do tempo"),
        ('3', Command::TogglePhotos, "Fotos"),
    ];
    for (key, command, label) in expected {
        assert_eq!(Command::from_shortcut(Shortcut::alt(key)), Some(command));
        // Ctrl+1/2/3 são do ZoomIt (PowerToys): não chegam ao Lamina e não são das faixas.
        assert_eq!(Command::from_shortcut(Shortcut::ctrl(key)), None);
        assert_eq!(command.label(), label);
        assert_eq!(command.menu(), Menu::View);
        assert_eq!(Command::from_name(command.name()), Some(command));
    }
    // Sem projeto (tela inicial), as faixas não existem (regra 6).
    assert!(!Editor::new().available(Command::TogglePhotos));
}

#[test]
fn pn_b3_a_janela_lembra_na_proxima_vez() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config").join("janela.json");
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load(&file));
    // Desde a versão 2 do estado da janela, a faixa de lâminas antiga começa desligada (JE).
    assert_eq!(open(&editor), [false, true, true]);
    assert!(editor.run(Command::TogglePhotos));
    // Fechar e abrir: o estado é lido de novo do arquivo.
    let mut next = editor_at(three_spreads(), 0);
    next.set_strips(StripsStore::load(&file));
    assert_eq!(open(&next), [false, true, false]);
    assert_eq!(next.take_notice(), None);
}

#[test]
fn pn_b4_arquivo_ilegivel_volta_ao_padrao_com_aviso() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, "isto não é json").unwrap();
    let store = StripsStore::load(&file);
    // O padrão novo (spec JE): painel escondido e faixa de lâminas antiga desligada.
    assert_eq!(store.strips, Strips::fresh());
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(store);
    assert_eq!(open(&editor), [false, true, true]);
    assert_eq!(editor.take_notice().as_deref(), Some(STRIPS_UNREADABLE));
    // A próxima mudança regrava o arquivo, que volta a ler.
    assert!(editor.run(Command::ToggleTimeline));
    assert!(!StripsStore::load(&file).strips.timeline);
}

#[test]
fn pn_b5_recolher_nao_perde_a_selecao() {
    let mut editor = editor_at(three_spreads(), 1);
    let photos: Vec<_> = editor.session().unwrap().project().photos[..2]
        .iter()
        .map(|p| p.id)
        .collect();
    editor.set_selection(photos.clone());
    editor.select_frame(Some(0));
    assert!(editor.run(Command::TogglePhotos));
    assert!(editor.run(Command::TogglePhotos));
    assert_eq!(editor.selection(), photos.as_slice());
    assert_eq!(editor.selected_frame(), Some(0));
    assert_eq!(editor.session().unwrap().current(), 1);
}

#[test]
fn pn_b6_cabecalhos_com_o_resumo_atual() {
    let photos = [H; 12];
    let mut editor = editor_at(built(spec(30.0, 30.0), &photos, 4), 0);
    let titles = strip_titles(editor.session().unwrap());
    assert_eq!(titles[0], "Lâminas · 4 lâminas (8 páginas)");
    assert_eq!(titles[2], "Fotos · 12 de 12 usadas");
    assert!(editor.run(Command::DeleteSpread));
    let session = editor.session().unwrap();
    let titles = strip_titles(session);
    assert_eq!(titles[0], "Lâminas · 3 lâminas (6 páginas)");
    let used = used_photos(session.project());
    assert!(used < 12);
    assert_eq!(titles[2], format!("Fotos · {used} de 12 usadas"));
    assert!(titles[1].starts_with("Linha do tempo · "));
}
