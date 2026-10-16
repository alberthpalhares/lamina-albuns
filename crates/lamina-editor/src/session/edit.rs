//! Edições do projeto pela sessão: cada uma entra no histórico com um rótulo (ADR 0005), e
//! desfazer e refazer levam a lâmina atual até a lâmina que mudou (spec TL, regra 9).

use lamina_core::Project;

use super::{Session, warnings_of};

impl Session {
    /// Aplica `change` no projeto como uma edição com `label`. Se der erro, nada muda. O rótulo
    /// ganha a lâmina atual ("Trocar layout · lâmina 3"), salvo quando já diz onde (spec HI,
    /// regra 1).
    pub fn edit<E>(
        &mut self,
        label: &str,
        change: impl FnOnce(&mut Project) -> Result<(), E>,
    ) -> Result<(), E> {
        let label = if label.contains(" · ") {
            label.to_owned()
        } else {
            format!("{label} · lâmina {}", self.current + 1)
        };
        self.history.try_apply(label, change)?;
        self.warnings = warnings_of(self.history.present());
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Desfaz a última edição; devolve o rótulo dela.
    pub fn undo(&mut self) -> Option<String> {
        let before = self.project().clone();
        let label = self.history.undo()?.to_owned();
        self.after_step(&before);
        Some(label)
    }

    /// Refaz a edição desfeita; devolve o rótulo dela.
    pub fn redo(&mut self) -> Option<String> {
        let before = self.project().clone();
        let label = self.history.redo()?.to_owned();
        self.after_step(&before);
        Some(label)
    }

    pub(super) fn after_step(&mut self, before: &Project) {
        let now = self.history.present();
        let changed = (0..now.spreads.len().max(before.spreads.len()))
            .find(|&i| now.spreads.get(i) != before.spreads.get(i));
        let last = now.spreads.len().saturating_sub(1);
        self.current = changed.unwrap_or(self.current).min(last);
        self.warnings = warnings_of(now);
    }
}
