//! Desfazer/refazer por snapshots imutáveis (ADR 0005).
//!
//! Cada edição clona o estado atual, altera a cópia e guarda o estado anterior. Como o
//! documento é pequeno (alguns KB), isso é barato e sempre correto: não existe "comando
//! inverso" para escrever errado. A lista de rótulos alimenta o painel de histórico.

use std::sync::Arc;

#[derive(Debug, Clone)]
struct Entry<T> {
    label: String,
    state: Arc<T>,
}

/// Histórico de edições de um valor `T` (normalmente um [`crate::Project`]).
#[derive(Debug, Clone)]
pub struct History<T> {
    present: Arc<T>,
    /// Estados anteriores; o último é o mais recente. O rótulo descreve a edição que saiu dele.
    past: Vec<Entry<T>>,
    /// Estados desfeitos, prontos para refazer; o último é o próximo.
    future: Vec<Entry<T>>,
    limit: usize,
}

impl<T: Clone> History<T> {
    /// Quantas edições ficam guardadas por padrão.
    pub const DEFAULT_LIMIT: usize = 200;

    pub fn new(initial: T) -> Self {
        Self::with_limit(initial, Self::DEFAULT_LIMIT)
    }

    pub fn with_limit(initial: T, limit: usize) -> Self {
        History {
            present: Arc::new(initial),
            past: Vec::new(),
            future: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// O estado atual.
    pub fn present(&self) -> &T {
        &self.present
    }

    /// Aplica uma edição que pode falhar. Se `edit` devolver `Err`, nada muda.
    pub fn try_apply<R, E>(
        &mut self,
        label: impl Into<String>,
        edit: impl FnOnce(&mut T) -> Result<R, E>,
    ) -> Result<R, E> {
        let mut next = T::clone(&self.present);
        let result = edit(&mut next)?;
        let previous = std::mem::replace(&mut self.present, Arc::new(next));
        self.past.push(Entry {
            label: label.into(),
            state: previous,
        });
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
        self.future.clear();
        Ok(result)
    }

    /// Aplica uma edição que não falha.
    pub fn apply<R>(&mut self, label: impl Into<String>, edit: impl FnOnce(&mut T) -> R) -> R {
        match self.try_apply(label, |state| {
            Ok::<R, std::convert::Infallible>(edit(state))
        }) {
            Ok(result) => result,
            Err(never) => match never {},
        }
    }

    /// Desfaz a última edição. Devolve o rótulo dela, ou `None` se não há o que desfazer.
    pub fn undo(&mut self) -> Option<&str> {
        let entry = self.past.pop()?;
        let current = std::mem::replace(&mut self.present, entry.state);
        self.future.push(Entry {
            label: entry.label,
            state: current,
        });
        self.future.last().map(|e| e.label.as_str())
    }

    /// Refaz a última edição desfeita. Devolve o rótulo dela.
    pub fn redo(&mut self) -> Option<&str> {
        let entry = self.future.pop()?;
        let current = std::mem::replace(&mut self.present, entry.state);
        self.past.push(Entry {
            label: entry.label,
            state: current,
        });
        self.past.last().map(|e| e.label.as_str())
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Rótulos das edições que podem ser desfeitas, da mais antiga para a mais recente.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.past.iter().map(|e| e.label.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desfaz_e_refaz() {
        let mut history = History::new(vec![1]);
        history.apply("adicionar 2", |v| v.push(2));
        history.apply("adicionar 3", |v| v.push(3));
        assert_eq!(history.present(), &vec![1, 2, 3]);

        assert_eq!(history.undo(), Some("adicionar 3"));
        assert_eq!(history.present(), &vec![1, 2]);
        assert_eq!(history.undo(), Some("adicionar 2"));
        assert_eq!(history.undo(), None);
        assert_eq!(history.present(), &vec![1]);

        assert_eq!(history.redo(), Some("adicionar 2"));
        assert_eq!(history.present(), &vec![1, 2]);
    }

    #[test]
    fn nova_edicao_apaga_o_refazer() {
        let mut history = History::new(0);
        history.apply("+1", |n| *n += 1);
        history.undo();
        assert!(history.can_redo());
        history.apply("+10", |n| *n += 10);
        assert!(!history.can_redo());
        assert_eq!(*history.present(), 10);
    }

    #[test]
    fn edicao_que_falha_nao_entra_no_historico() {
        let mut history = History::new(5);
        let result: Result<(), &str> = history.try_apply("falha", |n| {
            *n = 99;
            Err("não pode")
        });
        assert_eq!(result, Err("não pode"));
        assert_eq!(*history.present(), 5);
        assert!(!history.can_undo());
    }

    #[test]
    fn respeita_o_limite() {
        let mut history = History::with_limit(0, 3);
        for i in 1..=5 {
            history.apply(format!("passo {i}"), |n| *n = i);
        }
        let labels: Vec<_> = history.undo_labels().collect();
        assert_eq!(labels, ["passo 3", "passo 4", "passo 5"]);
    }
}
