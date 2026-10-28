//! As faixas de baixo e o painel da direita pela janela (spec FX): o botão que esconde e mostra as
//! três faixas e a largura do painel, ambos guardados em `janela.json`.

use super::Editor;
use crate::panel_width::{PANEL_WIDTH, panel_width};

impl Editor {
    /// O botão das faixas de baixo (regra 8): esconde as três ou abre as lembradas, e grava.
    pub(crate) fn toggle_bottom(&mut self) -> bool {
        self.strips.strips.toggle_bottom();
        self.save_strips();
        true
    }

    /// A largura do painel da direita numa janela de `window` px (regras 13 e 15).
    pub fn panel_width(&self, window: u32) -> u32 {
        panel_width(self.strips.strips.panel_width, window)
    }

    /// A borda do painel solta em `width` px (`None` = clique duplo, volta à de sempre): presa à
    /// faixa da janela, guardada e devolvida.
    pub fn set_panel_width(&mut self, width: Option<u32>, window: u32) -> u32 {
        let shown = panel_width(Some(width.unwrap_or(PANEL_WIDTH)), window);
        self.strips.strips.panel_width = Some(shown);
        self.save_strips();
        shown
    }
}
