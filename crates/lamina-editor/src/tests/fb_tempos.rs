//! As esperas da spec FB (regras 6 e 7): o silêncio depois de mudar o tamanho da janela e a
//! barra da lâmina que espera antes de sumir.

use crate::motion::{BAR_GRACE_MS, HoverGrace, Quiet, RESIZE_QUIET_MS};

#[test]
fn fb_b3_a_janela_muda_de_tamanho_em_varios_passos() {
    let mut quiet = Quiet::new(RESIZE_QUIET_MS);
    let mut fired = Vec::new();
    for now in [0, 40, 90, 130] {
        quiet.poke(now);
    }
    assert!(quiet.busy(200), "menos de 150 ms desde a última: não anima");
    assert!(!quiet.busy(290), "passou o silêncio: anima de novo");
    for now in (130..=400).step_by(10) {
        if quiet.settle(now) {
            fired.push(now);
        }
    }
    assert_eq!(fired, vec![280], "um redesenho só, no fim do silêncio");
}

#[test]
fn fb_b4_o_ponteiro_sai_da_lamina_e_chega_a_barra() {
    let mut grace = HoverGrace::default();
    grace.set(true, 0);
    grace.set(false, 0);
    assert!(grace.shown(60, false));
    grace.set(true, 120);
    assert!(grace.shown(300, false));
    assert!(grace.shown(900, false));
    assert_eq!(
        grace.due(),
        None,
        "com o ponteiro dentro, não há sumiço marcado"
    );
}

#[test]
fn fb_b5_o_ponteiro_sai_de_vez() {
    let mut grace = HoverGrace::default();
    grace.set(true, 0);
    grace.set(false, 0);
    assert!(grace.shown(BAR_GRACE_MS - 1, false));
    assert!(!grace.shown(BAR_GRACE_MS + 1, false));
    assert_eq!(grace.due(), Some(BAR_GRACE_MS));
    assert!(!grace.shown(100, true), "durante um arrasto, some na hora");
}
