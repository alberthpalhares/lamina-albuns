//! A caixa de mensagens do segundo plano (spec MV, regra 1): quem manda uma mensagem acorda a
//! linha da janela na hora (`invoke_from_event_loop`), em vez de esperar o temporizador.

use std::cell::RefCell;
use std::rc::Weak;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::state::{App, Msg};

/// O lado de quem manda: pode ir para qualquer linha.
#[derive(Debug, Clone)]
pub struct Mailbox(Sender<Msg>);

/// A janela já fechou: a mensagem não tem quem receba.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Closed;

type Inbox = Option<(Receiver<Msg>, Weak<RefCell<App>>)>;

thread_local! {
    // O lado de quem recebe, só na linha da janela.
    static INBOX: RefCell<Inbox> = const { RefCell::new(None) };
}

impl Mailbox {
    /// A caixa e o lado de quem recebe, para [`install`].
    pub fn new() -> (Mailbox, Receiver<Msg>) {
        let (tx, rx) = channel();
        (Mailbox(tx), rx)
    }

    /// Manda a mensagem e acorda a linha da janela.
    pub fn send(&self, msg: Msg) -> Result<(), Closed> {
        self.0.send(msg).map_err(|_| Closed)?;
        // Sem laço de eventos (janela fechando), o temporizador de reserva também não roda.
        let _ = slint::invoke_from_event_loop(drain);
        Ok(())
    }
}

/// Liga o lado de quem recebe ao estado da janela.
pub fn install(rx: Receiver<Msg>, app: &std::rc::Rc<RefCell<App>>) {
    INBOX.set(Some((rx, std::rc::Rc::downgrade(app))));
}

/// Quanto esperar para tentar de novo quando o estado da janela está ocupado.
const BUSY_RETRY: std::time::Duration = std::time::Duration::from_millis(10);

/// Trata todas as mensagens que chegaram. Se o estado estiver ocupado, tenta de novo uma vez, logo
/// depois; sem nada para entregar, a janela não gasta nada (spec LV, regra 7).
pub fn drain() {
    let Some(app) = INBOX.with_borrow(|inbox| inbox.as_ref().and_then(|(_, app)| app.upgrade()))
    else {
        return;
    };
    let Ok(mut app) = app.try_borrow_mut() else {
        slint::Timer::single_shot(BUSY_RETRY, drain);
        return;
    };
    let msgs: Vec<Msg> = INBOX.with_borrow(|inbox| {
        inbox
            .as_ref()
            .map(|(rx, _)| rx.try_iter().collect())
            .unwrap_or_default()
    });
    if msgs.is_empty() {
        return;
    }
    for msg in msgs {
        crate::new_album::message(&mut app, msg);
    }
    crate::images::after_arrivals(&mut app);
}

/// Roda `f` com o estado da janela daqui a `delay`, na linha da janela (por exemplo, retirar as
/// vagas que terminaram de esmaecer).
pub fn later(delay: std::time::Duration, f: fn(&mut App)) {
    slint::Timer::single_shot(delay, move || {
        let app = INBOX.with_borrow(|inbox| inbox.as_ref().and_then(|(_, app)| app.upgrade()));
        if let Some(app) = app
            && let Ok(mut app) = app.try_borrow_mut()
        {
            f(&mut app);
        }
    });
}

/// Como [`later`], de novo a cada `delay` até `f` devolver `true`; com o estado ocupado, tenta na
/// próxima vez. Janela fechada: para.
pub fn until(delay: std::time::Duration, f: fn(&mut App) -> bool) {
    slint::Timer::single_shot(delay, move || {
        let app = INBOX.with_borrow(|inbox| inbox.as_ref().and_then(|(_, app)| app.upgrade()));
        let done = match &app {
            Some(app) => app.try_borrow_mut().is_ok_and(|mut app| f(&mut app)),
            None => true,
        };
        if !done {
            until(delay, f);
        }
    });
}

/// Roda `f` com o estado da janela assim que ele estiver livre (a resposta de um diálogo); janela
/// fechada: não roda.
pub fn with_app(f: impl FnOnce(&mut App) + 'static) {
    let app = INBOX.with_borrow(|inbox| inbox.as_ref().and_then(|(_, app)| app.upgrade()));
    let Some(app) = app else {
        return;
    };
    match app.try_borrow_mut() {
        Ok(mut app) => f(&mut app),
        Err(_) => {
            slint::Timer::single_shot(std::time::Duration::from_millis(10), move || with_app(f));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lv_b11_sem_conferencia_repetida_da_caixa() {
        // A entrega de verdade passa pelo teste de fumaça da importação (TR); aqui fica a trava
        // de que o temporizador repetido não volta.
        let main = include_str!("main.rs");
        assert!(!main.contains("mailbox::drain,"));
        assert!(main.contains("single_shot(Duration::ZERO, mailbox::drain)"));
    }
}
