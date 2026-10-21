//! O painel de fotos pelo editor (spec PF): a aba mostrada, o laço, "Selecionar todas as fotos
//! mostradas", a primeira foto visível da grade e o zoom e a altura guardados em `janela.json`.
//! As contas da grade estão em [`crate::photo_grid`].

use std::collections::HashSet;

use lamina_core::PhotoId;

use super::Editor;
use crate::photo_grid::{clamp_zoom, default_height, shown_height, whole_px};
use crate::photos::{Filter, photo_list};

impl Editor {
    /// A aba do painel de fotos (Todas, Usadas ou Não usadas).
    pub fn photo_filter(&self) -> Filter {
        self.photo_filter
    }

    /// Troca a aba: a seleção continua, inclusive as fotos que a aba esconde (regra 11).
    pub fn set_photo_filter(&mut self, filter: Filter) {
        self.photo_filter = filter;
    }

    /// As fotos da aba, na ordem do painel (ordem de captura).
    pub fn shown_photos(&self) -> Vec<PhotoId> {
        self.session.as_ref().map_or_else(Vec::new, |s| {
            photo_list(s.project(), self.photo_filter)
                .into_iter()
                .map(|item| item.id)
                .collect()
        })
    }

    /// O laço terminou sobre `picked` (regra 9): sem tecla, a seleção passa a ser elas (vazia se
    /// o laço não cruzou nenhuma); com Ctrl ou Shift (`add`), elas se somam. A seleção fica na
    /// ordem do painel; as fotos que a aba esconde continuam no começo, como estavam.
    pub fn lasso_photos(&mut self, picked: &[PhotoId], add: bool) {
        let kept: &[PhotoId] = if add { &self.selection } else { &[] };
        let chosen: HashSet<PhotoId> = kept.iter().chain(picked).copied().collect();
        let shown = self.shown_photos();
        let visible: HashSet<PhotoId> = shown.iter().copied().collect();
        let mut selection: Vec<PhotoId> = kept
            .iter()
            .copied()
            .filter(|id| !visible.contains(id))
            .collect();
        selection.extend(shown.into_iter().filter(|id| chosen.contains(id)));
        self.selection = selection;
    }

    /// "Selecionar todas as fotos mostradas" (regra 10): a seleção são as fotos da aba, na ordem.
    pub(super) fn select_all_photos(&mut self) -> bool {
        let shown = self.shown_photos();
        if shown.is_empty() {
            return false;
        }
        self.selection = shown;
        true
    }

    /// A primeira foto visível da grade, guardada pela foto (não em pixels) para voltar a ela
    /// depois de esconder o painel ou de mudar o zoom ou a largura (regra 7).
    pub fn grid_anchor(&self) -> Option<PhotoId> {
        self.grid_anchor
    }

    pub fn set_grid_anchor(&mut self, photo: Option<PhotoId>) {
        self.grid_anchor = photo;
    }

    /// O zoom das miniaturas (64 a 240 px).
    pub fn photos_zoom(&self) -> u32 {
        clamp_zoom(i64::from(self.strips.strips.photos_zoom))
    }

    /// Muda o zoom (preso à faixa) e grava em `janela.json`. Devolve o zoom que ficou.
    pub fn set_photos_zoom(&mut self, zoom: i64) -> u32 {
        let zoom = clamp_zoom(zoom);
        if zoom != self.strips.strips.photos_zoom {
            self.strips.strips.photos_zoom = zoom;
            self.save_strips();
        }
        zoom
    }

    /// A altura guardada do painel de fotos (`None`: a padrão, duas linhas).
    pub fn photos_height(&self) -> Option<u32> {
        self.strips.strips.photos_height
    }

    /// A divisória foi solta (ou o duplo clique voltou à padrão): grava a altura que ficou na
    /// tela (regra 5).
    pub fn set_photos_height(&mut self, height: u32) {
        if self.strips.strips.photos_height != Some(height) {
            self.strips.strips.photos_height = Some(height);
            self.save_strips();
        }
    }

    /// Duplo clique na divisória: volta à altura padrão (duas linhas no zoom atual) e grava.
    pub fn reset_photos_height(&mut self) -> u32 {
        let height = whole_px(default_height(self.photos_zoom()));
        self.set_photos_height(height);
        height
    }

    /// A altura que a grade mostra numa janela de altura `window`, com `chrome` de barras e
    /// cabeçalhos (os limites da regra 5; o arquivo não muda).
    pub fn shown_photos_height(&self, window: f32, chrome: f32) -> f32 {
        let stored = self.photos_height().map(|h| h as f32);
        shown_height(stored, window, chrome, self.photos_zoom())
    }
}
