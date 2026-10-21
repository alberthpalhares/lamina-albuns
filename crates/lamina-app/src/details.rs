//! A aba Detalhes pela janela (spec AF, seção 4 e regra 10): as linhas que o editor monta para a
//! seleção, e a aba que se abre sozinha quando o editor pede (recorte, modo organizar e "Ajustar a
//! foto…"; clicar numa foto não troca a aba).

use lamina_editor::editor::Details;

use crate::DetailRow;
use crate::convert::model;
use crate::state::App;

/// A aba Detalhes, no painel da direita (a de partida; spec FL, regra 11).
pub(crate) const DETAILS_TAB: i32 = 1;

pub fn refresh(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    if app.editor.take_details_request() {
        w.set_panel_tab(DETAILS_TAB);
    }
    let (kind, rows, count) = match app.editor.details() {
        Details::Album(rows) => (0, rows, String::new()),
        Details::Photo(rows) => (1, rows, String::new()),
        Details::Text => (2, Vec::new(), String::new()),
        Details::Frames(n) => (3, Vec::new(), format!("{n} quadros")),
    };
    let rows: Vec<DetailRow> = rows
        .into_iter()
        .map(|(title, value)| DetailRow {
            title: title.into(),
            // Um caminho sem espaços não quebra a linha: um ponto de quebra invisível depois de
            // cada barra deixa o painel na largura dele.
            value: value
                .replace('\\', "\\\u{200B}")
                .replace('/', "/\u{200B}")
                .into(),
        })
        .collect();
    w.set_details(model(rows));
    w.set_details_kind(kind);
    w.set_details_count(count.into());
}
