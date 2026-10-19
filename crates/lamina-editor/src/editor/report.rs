//! Erros de uma edição viram o aviso de uma linha da janela, com a frase humana, e o detalhe vai
//! para o registro (spec EH, seção 4; spec SR, regra 5).

use super::Editor;
use crate::ErrorLog;
use crate::human::{Human, HumanMessage};

impl Editor {
    /// O registro de erros que a janela usa.
    pub fn set_log(&mut self, log: ErrorLog) {
        self.log = Some(log);
    }

    pub fn log(&self) -> Option<&ErrorLog> {
        self.log.as_ref()
    }

    /// Grava a mensagem no registro, se houver um.
    pub fn record(&self, message: &HumanMessage) {
        if let Some(log) = &self.log {
            log.error(message);
        }
    }

    /// O erro de uma edição: a frase e o que fazer viram o aviso, e tudo vai para o registro.
    pub(super) fn report(&mut self, error: &impl Human) -> String {
        let message = error.human();
        self.record(&message);
        let line = message.line();
        self.notice = Some(line.clone());
        line
    }
}
