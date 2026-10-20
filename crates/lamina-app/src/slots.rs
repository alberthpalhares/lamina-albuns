//! As vagas da lâmina grande na tela (spec MV, regra 4; ADR 0011, decisão 2): a lista da tela tem
//! uma linha por vaga, e uma mudança chega por `set_row_data`, `push` ou `remove`, nunca trocando a
//! lista inteira (a não ser em outra lâmina ou outro álbum). Assim a mesma instância da tela
//! continua mostrando a mesma foto. Quem decide as vagas é o `Slots` do editor.

use std::rc::Rc;

use lamina_editor::FrameView;
use lamina_editor::motion::{Place, Slot, Slots};
use slint::{Model, ModelRc, VecModel};

use crate::FrameData;

pub struct SlotRows {
    slots: Slots,
    model: Rc<VecModel<FrameData>>,
}

impl SlotRows {
    pub fn new() -> Self {
        SlotRows {
            slots: Slots::default(),
            model: Rc::new(VecModel::default()),
        }
    }

    /// A lista da tela (sempre a mesma).
    pub fn model(&self) -> ModelRc<FrameData> {
        ModelRc::from(self.model.clone())
    }

    /// Leva a lista às vagas dos quadros `frames`; `rows[k]` é o desenho do quadro `k`.
    pub fn sync(&mut self, frames: &[FrameView], rows: &[FrameData], place: Place, gesture: bool) {
        let reset = self.slots.update(frames, place, gesture);
        if reset {
            let all = self.slots.slots().iter().map(|s| row(s, rows, None));
            self.model.set_vec(all.collect::<Vec<_>>());
            return;
        }
        let known = self.model.row_count();
        for (i, s) in self.slots.slots().iter().enumerate() {
            if i < known {
                let old = self.model.row_data(i);
                let new = row(s, rows, old.clone());
                if old.as_ref() != Some(&new) {
                    self.model.set_row_data(i, new);
                }
            } else {
                self.model.push(row(s, rows, None));
            }
        }
        // Ainda sem esmaecer (o movimento entra no passo seguinte da MV): quem sai, sai já.
        for i in self.slots.sweep() {
            self.model.remove(i);
        }
    }
}

/// A linha da vaga: o desenho do quadro dela, ou o último desenho, apagando-se, se ela saiu.
fn row(slot: &Slot, rows: &[FrameData], old: Option<FrameData>) -> FrameData {
    let id = i32::try_from(slot.id).unwrap_or(i32::MAX);
    match slot.frame.and_then(|k| rows.get(k)) {
        Some(data) => FrameData {
            id,
            fly: slot.fly,
            born: slot.born,
            live: true,
            ..data.clone()
        },
        None => FrameData {
            id,
            index: -1,
            fly: false,
            born: false,
            live: false,
            ..old.unwrap_or_default()
        },
    }
}
