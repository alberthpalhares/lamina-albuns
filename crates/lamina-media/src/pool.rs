//! Quantas fotos são lidas ao mesmo tempo (spec TR, regra 1). Cada foto em andamento ocupa
//! cerca de 3 bytes por pixel mais o arquivo (uma de 45 MP, ~180 MB). As linhas globais do rayon
//! usariam todos os núcleos (8 a 16 num Mac), e a soma passava de vários GB.

use std::sync::OnceLock;

use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};

/// No máximo 4 fotos ao mesmo tempo, como as 4 lâminas da exportação.
pub const PHOTOS_AT_ONCE: usize = 4;

/// Fotos ao mesmo tempo: até [`PHOTOS_AT_ONCE`], nunca mais que os núcleos da máquina.
pub fn photos_at_once() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().min(PHOTOS_AT_ONCE))
}

/// As linhas das fotos, as mesmas para o programa inteiro: a importação e as imagens que faltam,
/// ao mesmo tempo, dividem as mesmas linhas. `None` se o sistema não deu as linhas.
fn pool() -> Option<&'static ThreadPool> {
    static POOL: OnceLock<Option<ThreadPool>> = OnceLock::new();
    POOL.get_or_init(|| {
        ThreadPoolBuilder::new()
            .num_threads(photos_at_once())
            .thread_name(|i| format!("lamina-fotos-{i}"))
            .build()
            .ok()
    })
    .as_ref()
}

/// `work` em cada item nas linhas das fotos, com os resultados na ordem da entrada (regra 12 do
/// AGENTS). Sem as linhas, vai um item de cada vez.
pub fn map_photos<T: Send, U: Send>(items: Vec<T>, work: impl Fn(T) -> U + Sync + Send) -> Vec<U> {
    match pool() {
        Some(pool) => pool.install(|| items.into_par_iter().map(work).collect()),
        None => items.into_iter().map(work).collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;

    #[test]
    fn tr_b2_no_maximo_quatro_fotos_ao_mesmo_tempo() {
        let (running, most) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let done = map_photos((0..32).collect(), |i: usize| {
            let now = running.fetch_add(1, Ordering::SeqCst) + 1;
            most.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(15));
            running.fetch_sub(1, Ordering::SeqCst);
            i * 10
        });
        // As linhas são as mesmas para o programa inteiro (e para os outros testes deste binário,
        // ao mesmo tempo): o que vale é o teto.
        let most = most.into_inner();
        assert!(most <= PHOTOS_AT_ONCE, "{most} fotos ao mesmo tempo");
        // A ordem da entrada, qualquer que seja a ordem de término.
        assert_eq!(done, (0..32).map(|i| i * 10).collect::<Vec<_>>());
    }

    #[test]
    fn nunca_mais_linhas_que_nucleos() {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        assert_eq!(photos_at_once(), cores.min(PHOTOS_AT_ONCE));
    }
}
