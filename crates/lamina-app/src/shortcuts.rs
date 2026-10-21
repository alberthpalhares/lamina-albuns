//! Ajuda → "Atalhos do teclado…" (spec AT): a lista do editor na janela. A lista, a busca e o
//! texto das teclas são do `lamina_editor::shortcuts`; aqui só a tradução para o Slint.

use lamina_editor::Platform;
use lamina_editor::shortcuts::{Group, NO_MATCH_HINT, no_match, search, sheet};
use slint::{ComponentHandle, SharedString};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, ShortcutGroup, ShortcutRow, ShortcutsUi};

/// Abre a lista com a busca vazia (regra 8), pelo menu, por Ctrl+/ ou por F1.
pub fn open(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ShortcutsUi>();
    ui.set_query(SharedString::new());
    show(&ui, "");
    ui.set_show(true);
}

/// Mostra o resultado da busca `query`: a lista em duas colunas e numa só, ou a frase de nada
/// achado.
fn show(ui: &ShortcutsUi<'_>, query: &str) {
    let groups = search(&sheet(Platform::current()), query);
    let (left, right) = groups.split_at(middle(&groups));
    ui.set_left(slint_groups(left));
    ui.set_right(slint_groups(right));
    ui.set_all(slint_groups(&groups));
    let empty = if groups.is_empty() {
        no_match(query)
    } else {
        String::new()
    };
    ui.set_empty(empty.into());
    ui.set_hint(NO_MATCH_HINT.into());
}

/// Onde a segunda coluna começa: as duas com alturas parecidas, sem mudar a ordem dos grupos.
fn middle(groups: &[Group]) -> usize {
    let size = |g: &Group| g.rows.len() + 2;
    let total: usize = groups.iter().map(size).sum();
    (0..=groups.len())
        .min_by_key(|&i| {
            let left: usize = groups[..i].iter().map(size).sum();
            left.abs_diff(total - left)
        })
        .unwrap_or(0)
}

fn slint_groups(groups: &[Group]) -> slint::ModelRc<ShortcutGroup> {
    model(
        groups
            .iter()
            .map(|g| ShortcutGroup {
                title: g.title.into(),
                rows: model(
                    g.rows
                        .iter()
                        .map(|r| ShortcutRow {
                            does: r.does.into(),
                            keys: model(r.keys.iter().map(|k| k.into()).collect()),
                            gesture: r.command.is_none(),
                        })
                        .collect(),
                ),
            })
            .collect(),
    )
}

pub fn connect(window: &AppWindow) {
    let ui = window.global::<ShortcutsUi>();
    let w = window.as_weak();
    ui.on_search(move |query| {
        if let Some(w) = w.upgrade() {
            show(&w.global::<ShortcutsUi>(), &query);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_duas_colunas_ficam_com_alturas_parecidas_e_na_ordem() {
        let groups = sheet(Platform::Windows);
        let split = middle(&groups);
        assert!(split > 0 && split < groups.len());
        let rows = |gs: &[Group]| gs.iter().map(|g| g.rows.len() + 2).sum::<usize>();
        let (left, right) = groups.split_at(split);
        let biggest = groups.iter().map(|g| g.rows.len() + 2).max().unwrap_or(0);
        assert!(rows(left).abs_diff(rows(right)) <= biggest);
        assert_eq!(middle(&[]), 0);
    }
}
