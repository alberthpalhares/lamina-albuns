//! Cenários da spec `painel-de-fotos` (PF): a grade, o zoom, a altura, o laço e o plano das
//! miniaturas. Os de seleção, abas e contador estão em `pf_selecao.rs`; os de velocidade, em
//! `pf_rapido.rs`.

use lamina_core::{FrameContent, PhotoId, Project, RelRect};

use super::tl_common::{H, spec};
use crate::photo_grid::{
    Grid, default_height, grid_rows, shown_height, wheel_zoom, zoom_at_pointer, zoom_keeping_top,
};
use crate::{Editor, StripsStore, ThumbList, ThumbPlan};

/// Um álbum sem lâminas com `count` fotos, em ordem de captura pelo nome.
pub fn photos(count: usize) -> Project {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    for i in 0..count {
        project.add_photo(format!("f{i:04}.jpg"), H.0, H.1);
    }
    project
}

/// Uma lâmina com um quadro para cada foto de `used` (a mesma foto pode vir duas vezes).
pub fn use_photos(project: &mut Project, used: &[PhotoId]) {
    let spread = project.add_spread();
    for (k, id) in used.iter().enumerate() {
        let rect = RelRect::new(0.05 * k as f64, 0.1, 0.04, 0.04).unwrap();
        let content = FrameContent::Photo {
            photo: *id,
            crop: Default::default(),
        };
        project.add_frame(spread, rect, content).unwrap();
    }
}

/// O editor com o álbum aberto (sem ir a lâmina nenhuma: pode não haver).
pub fn editor_with(project: Project) -> Editor {
    let mut editor = Editor::new();
    editor.adopt(project);
    editor
}

#[test]
fn pf_b1_linhas_pela_largura_e_pelo_zoom() {
    let g = Grid::new(1000, 1200.0, 96);
    assert_eq!((g.columns, g.rows(), g.row_range(90).len()), (11, 91, 10));
    assert!(g.named());
    assert_eq!(g.row_height(), 122.0);
    let g = Grid::new(1000, 1200.0, 240);
    assert_eq!((g.columns, g.rows()), (4, 250));
    let g = Grid::new(1000, 1200.0, 64);
    assert_eq!((g.columns, g.rows(), g.row_range(62).len()), (16, 63, 8));
    assert!(!g.named(), "com 64 px, sem nome");
    for width in [300.0, 100.0] {
        let g = Grid::new(1000, width, 240);
        assert_eq!((g.columns, g.rows()), (1, 1000));
    }
    // Mais estreito que uma peça: a peça encolhe para caber.
    assert_eq!(Grid::new(1000, 100.0, 240).tile, 84.0);
    let rows = grid_rows(1000, 1200.0, 96);
    assert_eq!(
        (rows.len(), rows[0].clone(), rows[90].clone()),
        (91, 0..11, 990..1000)
    );
}

#[test]
fn pf_b2_o_zoom_fica_em_janela_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, r#"{"laminas": false}"#).unwrap();
    let mut editor = editor_with(photos(3));
    editor.set_strips(StripsStore::load(&file));
    assert_eq!(editor.photos_zoom(), 96, "sem o campo, 96");
    std::fs::write(&file, r#"{"laminas": false, "fotos_zoom": 300}"#).unwrap();
    editor.set_strips(StripsStore::load(&file));
    assert_eq!(editor.photos_zoom(), 240, "fora da faixa, preso a ela");
    // O controle põe 160: o arquivo guarda, e os outros campos continuam.
    assert_eq!(editor.set_photos_zoom(160), 160);
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(saved["fotos_zoom"], 160);
    // O arquivo antigo abre a faixa Lâminas uma vez (spec FX, regra 11); o campo continua lá.
    assert_eq!(saved["laminas"], true);
    // A Ctrl+roda para cima, seis vezes, para em 240.
    let mut zoom = editor.photos_zoom();
    let mut seen = Vec::new();
    for _ in 0..6 {
        zoom = wheel_zoom(zoom, true);
        seen.push(zoom);
    }
    assert_eq!(seen, [176, 192, 208, 224, 240, 240]);
    assert_eq!(wheel_zoom(64, false), 64, "nem menos que 64");
}

#[test]
fn pf_b3_o_zoom_mantem_o_lugar() {
    let old = Grid::new(1000, 1200.0, 96);
    // A linha 40 no topo e o ponteiro sobre a foto 450, a 200 px do topo do painel.
    let scroll = 39.0 * old.row_height();
    let pointer = (992.0, 200.0);
    assert_eq!(old.top_row(scroll), 39);
    assert_eq!(old.row_of(449), old.top_row(scroll + pointer.1));
    assert_eq!(old.column_at(pointer.0), 449 % 11);
    // Ctrl+roda para cima: zoom 112, a foto 450 continua embaixo do ponteiro.
    let new = Grid::new(1000, 1200.0, 112);
    let after = zoom_at_pointer(&old, &new, scroll, pointer);
    let h = new.row_height();
    let center = new.row_of(449) as f32 * h + 8.0 + new.tile / 2.0 - after;
    assert!(
        (center - pointer.1).abs() < h / 2.0,
        "a foto foi para {center}"
    );
    // O controle põe 64: a linha da primeira foto visível antes fica no topo.
    let first = new.row_range(new.top_row(after)).start;
    let small = Grid::new(1000, 1200.0, 64);
    let top = zoom_keeping_top(&new, &small, after, 400.0);
    assert!(small.row_range(small.top_row(top)).contains(&first));
}

#[test]
fn pf_b4_altura_da_divisoria_e_os_limites() {
    let (window, chrome) = (1000.0, 100.0);
    assert_eq!(
        shown_height(Some(50.0), window, chrome, 96),
        130.0,
        "uma linha"
    );
    assert_eq!(shown_height(Some(900.0), window, chrome, 96), 700.0, "70%");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    let mut editor = editor_with(photos(3));
    editor.set_strips(StripsStore::load(&file));
    assert_eq!(
        editor.shown_photos_height(window, chrome),
        252.0,
        "sem o campo"
    );
    // Ao soltar, grava o que ficou na tela.
    editor.set_photos_height(700);
    let read = || StripsStore::load(&file).strips.photos_height;
    assert_eq!(read(), Some(700));
    // A janela cai para 600: a grade mostra 340 (a lâmina fica com 160), o arquivo não muda.
    assert_eq!(editor.shown_photos_height(600.0, chrome), 340.0);
    assert_eq!(read(), Some(700));
    // Duplo clique: volta a duas linhas e grava.
    assert_eq!(editor.reset_photos_height(), 252);
    assert_eq!(read(), Some(252));
    assert_eq!(default_height(96), 252.0);
    // Janela tão baixa que nem uma linha cabe: vale uma linha.
    assert_eq!(shown_height(Some(700.0), 300.0, chrome, 96), 130.0);
}

#[test]
fn pf_b5_laco() {
    let mut editor = editor_with(photos(1000));
    let shown = editor.shown_photos();
    let g = Grid::new(shown.len(), 1200.0, 96);
    let h = g.row_height();
    let ids = |picked: Vec<usize>| picked.into_iter().map(|i| shown[i]).collect::<Vec<_>>();
    // Do vão entre a 1ª e a 2ª coluna, na 1ª linha, até o meio da 3ª coluna, na 3ª linha.
    let first = g.lasso((108.0, 60.0), (264.0, 2.0 * h + 60.0));
    assert_eq!(first, [1, 2, 12, 13, 23, 24]);
    editor.lasso_photos(&ids(first), false);
    // Com Ctrl, um laço que cruza só a primeira peça da 5ª linha soma a foto 45.
    let fifth = g.lasso((20.0, 4.0 * h + 20.0), (40.0, 4.0 * h + 40.0));
    assert_eq!(fifth, [44]);
    editor.lasso_photos(&ids(fifth), true);
    let expected: Vec<PhotoId> = [1, 2, 12, 13, 23, 24, 44].map(|i| shown[i]).to_vec();
    assert_eq!(editor.selection(), expected);
    // Sem tecla, um laço só dentro de um vão deixa a seleção vazia.
    let none = g.lasso((105.0, 10.0), (110.0, 100.0));
    assert!(none.is_empty());
    editor.lasso_photos(&ids(none), false);
    assert!(editor.selection().is_empty());
}

#[test]
fn pf_b6_clique_ctrl_e_shift_na_grade() {
    let mut editor = editor_with(photos(1000));
    let shown = editor.shown_photos();
    let g = Grid::new(shown.len(), 1200.0, 96);
    assert_eq!(g.index_at(2, 7), Some(29), "linha 3, coluna 8 = foto 30");
    editor.click_photo(&shown, g.index_at(0, 4).unwrap(), false, false);
    editor.click_photo(&shown, 29, false, true);
    assert_eq!(editor.selection(), &shown[4..30], "fotos 5 a 30");
    editor.click_photo(&shown, 9, true, false);
    assert_eq!(editor.selection().len(), 25);
    assert!(!editor.selection().contains(&shown[9]));
}

#[test]
fn pf_b7_so_perto_da_tela_pede_miniatura() {
    let shown = editor_with(photos(1000)).shown_photos();
    let g = Grid::new(shown.len(), 1200.0, 96);
    let h = g.row_height();
    let order = |scroll: f32| -> Vec<PhotoId> {
        let visible = g.visible(scroll, 3.0 * h);
        g.thumb_order(visible)
            .into_iter()
            .map(|i| shown[i])
            .collect()
    };
    let mut plan = ThumbPlan::default();
    // No topo: as linhas 1 a 3, depois 4 a 6, e nada mais.
    let top = order(0.0);
    assert_eq!(top, shown[..66]);
    assert_eq!(plan.want(ThumbList::Browser, &top).load, top);
    // A linha 50 no topo: 50 a 52, depois 49, 53, 48, 54, 47 e 55; as fotos 1 a 66 saem.
    let rows: Vec<usize> = [50, 51, 52, 49, 53, 48, 54, 47, 55].map(|r| r - 1).to_vec();
    let expected: Vec<PhotoId> = rows
        .iter()
        .flat_map(|r| g.row_range(*r))
        .map(|i| shown[i])
        .collect();
    let at_50 = order(49.0 * h);
    assert_eq!(at_50, expected);
    let orders = plan.want(ThumbList::Browser, &at_50);
    assert_eq!(orders.load, at_50);
    assert_eq!(
        orders.cancel,
        shown[..66],
        "pedidas e ainda não lidas: canceladas"
    );
    // As visíveis chegam; uma linha abaixo, nenhuma foto já na tela é pedida de novo.
    for id in &at_50[..33] {
        assert!(plan.arrived(*id, true).0);
    }
    let next = plan.want(ThumbList::Browser, &order(50.0 * h));
    assert!(next.load.iter().all(|id| !at_50.contains(id)));
}

#[test]
fn pf_b12_esconder_e_mostrar_volta_a_mesma_foto() {
    let mut editor = editor_with(photos(1000));
    let shown = editor.shown_photos();
    let anchor = shown[299];
    editor.set_grid_anchor(Some(anchor));
    editor.click_photo(&shown, 10, false, false);
    editor.click_photo(&shown, 13, false, true);
    editor.set_photo_filter(crate::Filter::Unused);
    editor.set_photos_zoom(128);
    editor.set_photos_height(300);
    let before = (editor.selection().to_vec(), editor.photo_filter());
    for _ in 0..2 {
        assert!(editor.run(crate::Command::TogglePhotos));
    }
    assert_eq!((editor.selection().to_vec(), editor.photo_filter()), before);
    assert_eq!(
        (editor.photos_zoom(), editor.photos_height()),
        (128, Some(300))
    );
    assert_eq!(editor.grid_anchor(), Some(anchor));
    // A volta pela foto: com 11 ou com 9 colunas, a linha da foto 300 fica no topo.
    let index = shown.iter().position(|p| *p == anchor).unwrap();
    for (width, columns) in [(1504.0, 11), (1232.0, 9)] {
        let grid = Grid::new(shown.len(), width, 128);
        assert_eq!(grid.columns, columns);
        let scroll = grid.scroll_to(index, 300.0);
        assert!(grid.row_range(grid.top_row(scroll)).contains(&index));
    }
}

#[test]
fn pf_miniatura_no_tamanho_da_peca() {
    use crate::photo_grid::thumb_side;
    // Zoom 96 numa tela a 175%: 168 px de peça → miniatura de 192; a 100%, 128; zoom 240, 320.
    assert_eq!(thumb_side(96, 1.75), 192);
    assert_eq!(thumb_side(96, 1.0), 128);
    assert_eq!(thumb_side(240, 1.75), 320);
    assert_eq!(thumb_side(64, 0.5), 128, "escala menor que 1 vale 1");
    // A miniatura do disco (320) sai reduzida, na mesma proporção.
    let base = super::common::Base::with_photos(2, (600, 400));
    let project = base.project(1);
    let photo = &project.photos[0];
    let full = crate::load_thumb(photo, &base.cache()).unwrap();
    let small = crate::load_thumb_sized(photo, &base.cache(), 160).unwrap();
    assert_eq!(small.width().max(small.height()), 160);
    let ratio = |w: u32, h: u32| f64::from(w) / f64::from(h);
    let (a, b) = (
        ratio(full.width(), full.height()),
        ratio(small.width(), small.height()),
    );
    assert!((a - b).abs() < 0.02, "{a} e {b}");
    let same = crate::load_thumb_sized(photo, &base.cache(), 1000).unwrap();
    assert_eq!(same.dimensions(), full.dimensions());
}
