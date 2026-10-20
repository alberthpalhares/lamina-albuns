//! Movimento e fluidez (spec MV, ADR 0011): o editor planeja, a tela anima. Aqui ficam as
//! contas sem janela: a medição de tempos e, a cada entrega, o resto da spec.

pub mod drag;
pub mod durations;
pub mod navigation;
pub mod rows;
pub mod slots;
pub mod timings;

pub use drag::{GHOST_PX, Ghost, Reorder, drop_target, frame_ghost, photo_ghost, reorder};
pub use durations::Durations;
pub use navigation::{Navigator, RAPID, Transition};
pub use rows::{
    PanelKey, PhotoRow, RowOp, StripRow, TimelineKey, panel_key, photo_rows, row_ops, strip_rows,
    timeline_rows,
};
pub use slots::{Place, Slot, SlotContent, Slots};
pub use timings::{BURST_GAP, TIMINGS_VAR, Timings};
