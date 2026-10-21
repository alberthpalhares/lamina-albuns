//! O rolo de lâminas (spec RL, regras 5 a 9; ADR 0012, decisões 4 e 5): os níveis das imagens,
//! o que o rolo mostra, os pedidos ao trabalhador e o que volta dele. O plano fica em
//! `roll_plan.rs` e as revisões em `roll_revisions.rs`. Sem janela e sem relógio: a mesma
//! sequência dá sempre o mesmo resultado.

use std::ops::RangeInclusive;

use lamina_core::SpreadId;

use crate::screens::Arrival;

pub use super::roll_plan::RollPlan;
pub use super::roll_revisions::Revisions;

/// Teto das imagens guardadas, em bytes (regra 9).
pub const ROLL_BYTES: usize = 64 * 1024 * 1024;
/// Largura da imagem pequena, feita só com miniaturas (regra 5).
pub const SMALL_WIDTH: u32 = 384;
/// Teto da largura da imagem de tela (regra 5).
pub const SCREEN_MAX: u32 = 2048;
/// Mudança de largura que faz pedir a imagem de tela de novo (regra 25).
const WIDTH_SLACK: f64 = 0.10;

/// O nível de uma imagem de lâmina (regra 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// 384 px de largura, só com miniaturas: rápida, para nada ficar só com o número.
    Small,
    /// A largura física do lugar, com teto de 2.048 px.
    Screen,
}

/// O que o rolo mostra agora: a lâmina no meio da área, as lâminas à vista, a lâmina atual (que
/// é viva e só pede a imagem de tela depois de 300 ms sem edição) e a largura física do lugar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollView {
    pub center: usize,
    pub visible: RangeInclusive<usize>,
    pub current: usize,
    pub current_idle: bool,
    pub width: u32,
}

/// Um pedido ao trabalhador do rolo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollAsk {
    /// O álbum: pedidos de um álbum anterior são descartados (RL-B22).
    pub epoch: u64,
    pub spread: SpreadId,
    pub index: usize,
    pub tier: Tier,
    pub revision: u64,
    /// Largura da imagem, em pixels.
    pub width: u32,
}

/// Uma imagem pronta, do jeito que o plano a conta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollGot {
    pub ask: RollAsk,
    /// Feita com todas as imagens de tela que pedia (regra 4).
    pub complete: bool,
    pub bytes: usize,
}

/// O que fazer com uma imagem que chegou e quais imagens guardadas saem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    pub arrival: Arrival,
    pub evict: Vec<SpreadId>,
}

/// A imagem de largura `have` serve para o lugar de largura `want` (menos de 10% de diferença).
pub(super) fn close(have: u32, want: u32) -> bool {
    let have = f64::from(have.max(1));
    (f64::from(want) - have).abs() / have <= WIDTH_SLACK
}
