//! O painel temporário (spec JE, regras 5 e 6): com o painel da direita escondido, ele aparece
//! sozinho enquanto for preciso ("Ajustar a foto…", escrever um texto, o modo organizar), sem
//! mudar a escolha guardada em `janela.json`.

use lamina_core::{FrameId, SpreadId};

use super::Editor;

/// O que pediu o painel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelNeed {
    /// "Ajustar a foto…", T ou duplo clique num texto: enquanto aquele quadro continuar
    /// selecionado, naquela lâmina.
    Frame { spread: SpreadId, frame: FrameId },
    /// O modo organizar, enquanto ele estiver aberto.
    Organize,
}

impl Editor {
    /// O painel da direita aparece: a escolha guardada ou um pedido em curso (regra 5). Um pedido
    /// que acabou (o quadro desmarcado ou trocado, outra lâmina, fora do modo, o álbum fechado)
    /// é esquecido aqui e não volta.
    pub fn panel_shown(&self) -> bool {
        if self.strips.strips.panel {
            return true;
        }
        let Some(need) = self.panel_need.get() else {
            return false;
        };
        let alive = self.need_alive(need);
        if !alive {
            self.panel_need.set(None);
        }
        alive
    }

    fn need_alive(&self, need: PanelNeed) -> bool {
        match need {
            PanelNeed::Organize => self.organizing(),
            PanelNeed::Frame { spread, frame } => {
                self.selected_frame_key() == Some((spread, frame)) && !self.organizing()
            }
        }
    }

    /// A lâmina atual e o quadro selecionado nela.
    fn selected_frame_key(&self) -> Option<(SpreadId, FrameId)> {
        let s = self.session.as_ref()?;
        let spread = s.project().spreads.get(s.current())?;
        let frame = spread.frames.get(self.selected_frame?)?;
        Some((spread.id, frame.id))
    }

    /// Pede o painel para o quadro selecionado. No modo organizar, o pedido do modo continua.
    pub(super) fn ask_panel_for_selected(&mut self) {
        if self.organizing() {
            return;
        }
        if let Some((spread, frame)) = self.selected_frame_key() {
            self.panel_need
                .set(Some(PanelNeed::Frame { spread, frame }));
        }
    }

    /// Entrar no modo organizar pede o painel; sair encerra o pedido.
    pub(super) fn ask_panel_for_mode(&mut self, on: bool) {
        self.panel_need.set(on.then_some(PanelNeed::Organize));
    }

    /// O botão Painel e Alt+4 alternam o que se vê (regra 6). Com o painel aparecendo por um
    /// pedido, escondem e encerram o pedido, sem gravar; senão, mudam a escolha e gravam.
    pub(super) fn toggle_panel(&mut self) -> bool {
        let shown = self.panel_shown();
        self.panel_need.set(None);
        if shown && !self.strips.strips.panel {
            return true;
        }
        self.strips.strips.panel = !shown;
        self.save_strips();
        true
    }
}
