//! Edições do projeto pela sessão: cada uma entra no histórico com um rótulo (ADR 0005), e
//! desfazer e refazer levam a lâmina atual até a lâmina que mudou (spec TL, regra 9).

use lamina_core::Project;
use lamina_export::preflight;

use super::{Session, missing_of, warnings_of};

impl Session {
    /// Aplica `change` no projeto como uma edição com `label`. Se der erro, nada muda. O rótulo
    /// ganha a lâmina atual ("Trocar layout · lâmina 3"), salvo quando já diz onde (spec HI,
    /// regra 1).
    pub fn edit<E>(
        &mut self,
        label: &str,
        change: impl FnOnce(&mut Project) -> Result<(), E>,
    ) -> Result<(), E> {
        let prefix = self.label_prefix;
        let label = if label.contains(" · ") {
            format!("{prefix}{label}")
        } else {
            format!("{prefix}{label} · lâmina {}", self.current + 1)
        };
        let before = self.project_shared();
        self.history.try_apply(label, change)?;
        self.update_warnings(&before);
        self.merge = None;
        Ok(())
    }

    /// Como [`Session::edit`], mas emenda a edição anterior quando ela tem a mesma `key` e nada
    /// aconteceu no meio: digitar ou mover aos poucos vira um passo só (spec TX, regra 10).
    pub fn edit_merging<E>(
        &mut self,
        label: &str,
        key: &str,
        change: impl FnOnce(&mut Project) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.merge.as_deref() == Some(key) && self.history.can_undo() {
            let before = self.project_shared();
            self.history.try_amend(change)?;
            self.update_warnings(&before);
            return Ok(());
        }
        self.edit(label, change)?;
        self.merge = Some(key.to_owned());
        Ok(())
    }

    /// A próxima edição não emenda a anterior (trocou de campo, de quadro ou de comando).
    pub fn end_merge(&mut self) {
        self.merge = None;
    }

    /// As próximas edições começam o rótulo com `prefix` ("IA: "; "" volta ao normal).
    pub fn set_label_prefix(&mut self, prefix: &'static str) {
        self.label_prefix = prefix;
        self.merge = None;
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Desfaz a última edição; devolve o rótulo dela.
    pub fn undo(&mut self) -> Option<String> {
        let before = self.project_shared();
        let label = self.history.undo()?.to_owned();
        self.after_step(&before);
        Some(label)
    }

    /// Refaz a edição desfeita; devolve o rótulo dela.
    pub fn redo(&mut self) -> Option<String> {
        let before = self.project_shared();
        let label = self.history.redo()?.to_owned();
        self.after_step(&before);
        Some(label)
    }

    pub(super) fn after_step(&mut self, before: &Project) {
        self.merge = None;
        let now = self.history.present();
        let changed = (0..now.spreads.len().max(before.spreads.len()))
            .find(|&i| now.spreads.get(i) != before.spreads.get(i));
        let last = now.spreads.len().saturating_sub(1);
        self.current = changed.unwrap_or(self.current).min(last);
        // Desfazer "Reencontrar fotos" volta aos caminhos antigos (spec EI). Desfazer um ajuste de
        // brilho não muda caminho nenhum: não olha o disco (spec LV, regra 5).
        let paths = now.photos.iter().map(|p| &p.path);
        if !paths.eq(before.photos.iter().map(|p| &p.path)) {
            self.missing = missing_of(now);
        }
        self.update_warnings(before);
    }

    /// Refaz os avisos de pré-impressão: só das lâminas que mudaram quando dá (digitar num texto
    /// não desenha de novo os textos do álbum inteiro; spec LV, regra 2).
    fn update_warnings(&mut self, before: &Project) {
        let now = self.history.present();
        let Some(changed) = now.spreads_changed_since(before) else {
            self.warnings = warnings_of(now);
            return;
        };
        if changed.is_empty() {
            return;
        }
        self.warnings
            .retain(|warning| !changed.contains(&(warning.spread - 1)));
        self.warnings.extend(preflight(now, &changed));
        // `sort_by_key` é estável: os avisos de cada lâmina continuam na ordem dos quadros.
        self.warnings.sort_by_key(|warning| warning.spread);
    }
}
