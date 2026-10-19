//! Edições do projeto pela sessão: cada uma entra no histórico com um rótulo (ADR 0005), e
//! desfazer e refazer levam a lâmina atual até a lâmina que mudou (spec TL, regra 9).

use lamina_core::Project;

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
        let label = if label.contains(" · ") {
            label.to_owned()
        } else {
            format!("{label} · lâmina {}", self.current + 1)
        };
        self.history.try_apply(label, change)?;
        self.warnings = warnings_of(self.history.present());
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
            self.history.try_amend(change)?;
            self.warnings = warnings_of(self.history.present());
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
        self.merge = None;
        let now = self.history.present();
        let changed = (0..now.spreads.len().max(before.spreads.len()))
            .find(|&i| now.spreads.get(i) != before.spreads.get(i));
        let last = now.spreads.len().saturating_sub(1);
        self.current = changed.unwrap_or(self.current).min(last);
        self.warnings = warnings_of(now);
        // Desfazer "Reencontrar fotos" volta aos caminhos antigos (spec EI).
        if now.photos != before.photos {
            self.missing = missing_of(now);
        }
    }
}
