//! As falhas da beta.5 (spec FB): as contas que fechavam o Lamina por um arredondamento.

use crate::group::fit;

#[test]
fn fb_b2_quadro_do_tamanho_do_corte_arrastado_a_mao_nao_derruba_o_lamina() {
    // Corte começando em 3,175 mm: (3,175 + largura) - largura fica um fio abaixo de 3,175, e o
    // `clamp` com o mínimo acima do máximo entrava em pânico.
    let (left, width) = (3.175, 510.571_428_571_428_56);
    let right_limit = (left + width) - width;
    assert!(right_limit < left);
    assert_eq!(fit(5.0, left, right_limit), left);
    assert_eq!(fit(1.0, left, right_limit), left);
    assert_eq!(fit(4.0, 3.0, 5.0), 4.0);
}
