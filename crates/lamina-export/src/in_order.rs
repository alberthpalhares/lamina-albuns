//! Trabalho em paralelo com a entrega na ordem da entrada (regra 12; spec LV, regra 14): sempre
//! `at_once` itens em andamento, sem esperar o mais lento de um lote.

use std::collections::BTreeMap;
use std::sync::mpsc::channel;
use std::sync::{Condvar, Mutex, PoisonError};

/// Quantos itens prontos podem esperar um mais lento, por item em andamento: limita a memória
/// quando a primeira lâmina demora muito mais que as outras.
const WAITING: usize = 2;

#[derive(Default)]
struct Turn {
    next: usize,
    delivered: usize,
    stop: bool,
}

/// Se quem o segura entrar em pânico, ninguém fica esperando uma entrega que não vem: os outros
/// trabalhadores param e o pânico sobe pelo `scope` (auditoria da spec LV).
struct StopOnPanic<'a>(&'a Mutex<Turn>, &'a Condvar);

impl Drop for StopOnPanic<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).stop = true;
            self.1.notify_all();
        }
    }
}

/// Roda `work` em cada item, até `at_once` ao mesmo tempo, e entrega os resultados a `got` na
/// ordem de `items`. Quando `got` devolve `false`, nada novo começa; o que já estava em andamento
/// termina e ainda é entregue (quem falhou guarda os arquivos das outras lâminas).
pub(crate) fn in_order<T: Sync, R: Send>(
    items: &[T],
    at_once: usize,
    work: &(dyn Fn(&T) -> R + Sync),
    got: &mut dyn FnMut(R) -> bool,
) {
    let at_once = at_once.max(1);
    let (turn, freed) = (Mutex::new(Turn::default()), Condvar::new());
    let (tx, rx) = channel();
    let lock = || turn.lock().unwrap_or_else(PoisonError::into_inner);
    std::thread::scope(|scope| {
        for _ in 0..at_once {
            let tx = tx.clone();
            let (turn, freed) = (&turn, &freed);
            scope.spawn(move || {
                let _guard = StopOnPanic(turn, freed);
                loop {
                    let index = {
                        let mut t = turn.lock().unwrap_or_else(PoisonError::into_inner);
                        while !t.stop && t.next >= t.delivered + at_once * (1 + WAITING) {
                            t = freed.wait(t).unwrap_or_else(PoisonError::into_inner);
                        }
                        if t.stop || t.next >= items.len() {
                            return;
                        }
                        t.next += 1;
                        t.next - 1
                    };
                    if tx.send((index, work(&items[index]))).is_err() {
                        return;
                    }
                }
            });
        }
        drop(tx);
        let _guard = StopOnPanic(&turn, &freed);
        let (mut ready, mut delivered) = (BTreeMap::new(), 0);
        for (index, result) in rx {
            ready.insert(index, result);
            while let Some(result) = ready.remove(&delivered) {
                let keep = got(result);
                delivered += 1;
                let mut t = lock();
                t.delivered = delivered;
                t.stop |= !keep;
                freed.notify_all();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn lv_b19_a_entrega_segue_a_ordem_mesmo_com_tempos_diferentes() {
        let items: Vec<u64> = (0..12).collect();
        let mut out = Vec::new();
        in_order(
            &items,
            3,
            &|n: &u64| {
                // Os primeiros demoram mais que os de depois.
                std::thread::sleep(Duration::from_millis(30u64.saturating_sub(*n * 3)));
                *n
            },
            &mut |n| {
                out.push(n);
                true
            },
        );
        assert_eq!(out, items);
    }

    #[test]
    fn lv_b19_parar_nao_comeca_nada_novo_e_entrega_o_que_estava_em_andamento() {
        let items: Vec<u32> = (0..100).collect();
        let mut out = Vec::new();
        in_order(&items, 2, &|n: &u32| *n, &mut |n| {
            out.push(n);
            n < 3
        });
        assert!(out.starts_with(&[0, 1, 2, 3]));
        assert!(out.len() < 100);
        assert!(out.windows(2).all(|w| w[0] + 1 == w[1]));
    }

    #[test]
    fn lv_b19_panico_numa_lamina_sobe_em_vez_de_travar() {
        let items: Vec<u32> = (0..50).collect();
        let result = std::panic::catch_unwind(|| {
            in_order(
                &items,
                2,
                &|n: &u32| {
                    assert_ne!(*n, 0, "a primeira lâmina falha");
                    *n
                },
                &mut |_| true,
            );
        });
        // Chegar aqui já prova que não travou; `unwrap_err` prova que o pânico subiu.
        let _panic = result.unwrap_err();
    }

    #[test]
    fn lv_b19_lista_vazia_nao_entrega_nada() {
        let mut calls = 0;
        in_order(&[] as &[u8], 4, &|n: &u8| *n, &mut |_| {
            calls += 1;
            true
        });
        assert_eq!(calls, 0);
    }
}
