//! Fotos de tela fora da linha da janela (spec MV, regra 1): o plano dos pedidos, o descarte do
//! pedido velho e a foto nítida que espera o fim do deslize (MV-B18, MV-B19 e MV-B26).

use std::collections::BTreeSet;

use lamina_core::{FrameContent, PhotoId, Project};

use super::tl_common::{H, V, built, spec};
use crate::screens::{Arrival, ScreenPlan};

/// Álbum de `spreads` lâminas com 4 fotos cada (uma vertical a cada quatro).
fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

/// As fotos da lâmina `index`, na ordem dos quadros.
fn photos_of(project: &Project, index: usize) -> Vec<PhotoId> {
    project.spreads[index]
        .frames
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Photo { photo, .. } => Some(*photo),
            _ => None,
        })
        .collect()
}

#[test]
fn mv_b18_plano_traz_a_lamina_atual_primeiro_e_depois_as_vizinhas() {
    let project = album(8);
    let mut plan = ScreenPlan::default();
    // A lâmina 3 da tela é a de índice 2.
    let asked = plan.go(&project, 2);
    let mut expected = photos_of(&project, 2);
    expected.extend(photos_of(&project, 1));
    expected.extend(photos_of(&project, 3));
    assert_eq!(asked.photos, expected);
    assert_eq!(asked.now, photos_of(&project, 2).len());
    assert!(asked.release.is_empty());
    // A lâmina vem com as miniaturas: nenhuma foto nítida antes de chegar.
    assert!(expected.iter().all(|id| !plan.is_sharp(*id)));
    // O que já foi pedido não é pedido de novo.
    assert!(plan.go(&project, 2).photos.is_empty());
}

#[test]
fn mv_b18_lamina_da_ponta_so_tem_uma_vizinha() {
    let project = album(4);
    let mut plan = ScreenPlan::default();
    let asked = plan.go(&project, 0);
    let mut expected = photos_of(&project, 0);
    expected.extend(photos_of(&project, 1));
    assert_eq!(asked.photos, expected);
}

#[test]
fn mv_b19_pedido_velho_que_chega_e_descartado() {
    let project = album(8);
    let mut plan = ScreenPlan::default();
    plan.go(&project, 2);
    // Vai à lâmina 7 antes de as fotos da 3 chegarem.
    let asked = plan.go(&project, 6);
    assert_eq!(asked.now, photos_of(&project, 6).len());
    for id in photos_of(&project, 2) {
        assert_eq!(plan.arrived(id, false), Arrival::Discard, "{id:?}");
        assert!(!plan.is_sharp(id));
    }
    for id in photos_of(&project, 6) {
        assert_eq!(plan.arrived(id, false), Arrival::Show);
        assert!(plan.is_sharp(id));
    }
}

#[test]
fn mv_b19_fotos_que_deixam_de_ser_vizinhas_sao_soltas() {
    let project = album(8);
    let mut plan = ScreenPlan::default();
    let asked = plan.go(&project, 2);
    for id in &asked.photos {
        assert_eq!(plan.arrived(*id, false), Arrival::Show);
    }
    let moved = plan.go(&project, 6);
    let released: BTreeSet<PhotoId> = moved.release.into_iter().collect();
    let around: BTreeSet<PhotoId> = (1..=3).flat_map(|i| photos_of(&project, i)).collect();
    assert_eq!(released, around);
    assert!(around.iter().all(|id| !plan.is_sharp(*id)));
}

#[test]
fn mv_b26_foto_nitida_espera_o_fim_do_deslize() {
    let project = album(8);
    let mut plan = ScreenPlan::default();
    let asked = plan.go(&project, 2);
    let (first, second) = (asked.photos[0], asked.photos[1]);
    // Durante o deslize, fica guardada.
    assert_eq!(plan.arrived(first, true), Arrival::Hold);
    assert!(!plan.is_sharp(first));
    assert_eq!(plan.slide_ended(), vec![first]);
    assert!(plan.is_sharp(first));
    assert!(plan.slide_ended().is_empty());
    // Sem deslize, entra na hora.
    assert_eq!(plan.arrived(second, false), Arrival::Show);
    assert!(plan.is_sharp(second));
}

#[test]
fn mv_b26_foto_guardada_que_deixa_de_ser_vizinha_nao_entra() {
    let project = album(8);
    let mut plan = ScreenPlan::default();
    let first = plan.go(&project, 2).photos[0];
    assert_eq!(plan.arrived(first, true), Arrival::Hold);
    plan.go(&project, 6);
    assert!(plan.slide_ended().is_empty());
    assert!(!plan.is_sharp(first));
}

#[test]
fn foto_sem_imagem_de_tela_so_e_pedida_de_novo_quando_o_cache_muda() {
    let project = album(4);
    let mut plan = ScreenPlan::default();
    let first = plan.go(&project, 1).photos[0];
    plan.failed(first);
    assert!(!plan.go(&project, 1).photos.contains(&first));
    plan.recheck();
    assert_eq!(plan.go(&project, 1).photos, vec![first]);
}

#[test]
fn mv_b18_mudar_de_lamina_pede_de_novo_o_que_nao_chegou_na_ordem_nova() {
    let project = album(8);
    let mut plan = ScreenPlan::default();
    plan.go(&project, 2);
    // Nada chegou; na lâmina 4, as fotos dela (já pedidas como vizinhas) vêm primeiro.
    let asked = plan.go(&project, 3);
    let mut expected = photos_of(&project, 3);
    expected.extend(photos_of(&project, 2));
    expected.extend(photos_of(&project, 4));
    assert_eq!(asked.photos, expected);
}
