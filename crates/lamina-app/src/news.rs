//! "O que há de novo" (spec NV): as notas e as ilustrações vão dentro do programa (regra 11), e a
//! janela mostra o que o `lamina_editor::updates::news` decide. Aqui só a tradução para o Slint.

use lamina_editor::updates::news::{
    NewsEntry, NewsPage, NewsSection, after_update, entries, from_help, illustration, next, parse,
};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, NewsEntryData, NewsUi};

/// As notas de todas as versões, as mesmas da página da versão no GitHub.
const NOTES: &str = include_str!("../dist/novidades.md");

/// As ilustrações de `dist/novidades/` (geradas pelo `build.rs`).
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/novidades.rs"));
}

/// `LAMINA_NOVIDADES=<versão>`: o Lamina compilado abre a janela como se viesse dessa versão, sem
/// gravar nada (seção 3).
const SIMULATE: &str = "LAMINA_NOVIDADES";

fn sections() -> Vec<NewsSection> {
    parse(NOTES)
}

/// Na abertura, antes de a janela aparecer (regra 3): lembra a versão e, se o Lamina instalado
/// acabou de atualizar e há notas, abre a janela.
pub fn on_start(app: &mut App) {
    let step = match std::env::var(SIMULATE) {
        Ok(previous) => app.updates.simulate_start(&previous),
        Err(_) => app.updates.on_start(),
    };
    if let Some(notice) = step.notice {
        app.notice = notice;
    }
    let Some(previous) = step.news_since else {
        return;
    };
    if let Some(page) = after_update(&sections(), &previous, app.updates.current()) {
        show(app, &page);
    }
}

/// Ajuda → "O que há de novo…" (regra 5).
pub fn open_help(app: &App) {
    show(app, &from_help(&sections(), app.updates.current()));
}

fn show(app: &App, page: &NewsPage) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewsUi>();
    ui.set_title(page.title.clone().into());
    ui.set_subtitle(page.subtitle.clone().into());
    ui.set_empty(page.empty.clone().into());
    ui.set_entries(model(entries(page).iter().map(entry_data).collect()));
    ui.set_selected(0);
    ui.set_show(true);
}

fn entry_data(entry: &NewsEntry) -> NewsEntryData {
    let item = &entry.item;
    let image = item.image.as_deref().and_then(picture);
    NewsEntryData {
        heading: entry.heading.clone().unwrap_or_default().into(),
        intro: entry.intro.clone().into(),
        current: entry.current,
        label: entry.label.clone().into(),
        title: item.title.clone().unwrap_or_default().into(),
        text: item.text.clone().into(),
        try_it: item.try_it.clone().unwrap_or_default().into(),
        command: item.command.clone().unwrap_or_default().into(),
        has_image: image.is_some(),
        image: image.unwrap_or_default(),
    }
}

/// A ilustração embutida de nome `name`, pronta para a janela.
fn picture(name: &str) -> Option<Image> {
    let (_, bytes) = embedded::IMAGES.iter().find(|(n, _)| *n == name)?;
    let decoded = illustration(bytes)?;
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        &decoded.rgba,
        decoded.width,
        decoded.height,
    );
    Some(Image::from_rgba8(buffer))
}

pub fn connect(window: &AppWindow) {
    let w = window.as_weak();
    window.global::<NewsUi>().on_next(move || {
        if let Some(w) = w.upgrade() {
            let ui = w.global::<NewsUi>();
            let count = slint::Model::row_count(&ui.get_entries());
            let at = usize::try_from(ui.get_selected()).unwrap_or(0);
            ui.set_selected(i32::try_from(next(at, count)).unwrap_or(0));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_notas_embutidas_leem_e_toda_ilustracao_citada_esta_embutida() {
        let all = sections();
        assert!(!all.is_empty());
        assert!(all.iter().all(|s| !s.items.is_empty()));
        for item in all.iter().flat_map(|s| &s.items) {
            if let Some(name) = &item.image {
                assert!(picture(name).is_some(), "{name} não está embutida");
            }
        }
    }
}
