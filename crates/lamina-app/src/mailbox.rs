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

/// Trata todas as mensagens que chegaram. Se o estado estiver ocupado, elas esperam a próxima.
pub fn drain() {
    let Some(app) = INBOX.with_borrow(|inbox| inbox.as_ref().and_then(|(_, app)| app.upgrade()))
    else {
        return;
    };
    let Ok(mut app) = app.try_borrow_mut() else {
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
