//! Movimento e fluidez (spec MV, ADR 0011): o editor planeja, a tela anima. Aqui ficam as
//! contas sem janela: a medição de tempos e, a cada entrega, o resto da spec.

pub mod timings;

pub use timings::{BURST_GAP, TIMINGS_VAR, Timings};
