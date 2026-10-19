//! Recorte pela IA (spec IA, `recortar`; spec RC): o recorte que o modo recorte deixaria com o
//! zoom e o centro pedidos, numa edição só. Zoom ou centro fora da faixa recusam sem mudar nada.

use lamina_core::{CoreError, Crop, FrameContent};

use super::{Agent, CropCall, ToolError, ToolReply, edit_on, photo_frame, session, spread_index};
use crate::editor::{crop_for, set_crop};
use crate::frame_form::decimal;
use crate::{Editor, EditorError};

impl Agent {
    pub(super) fn crop(
        &mut self,
        editor: &mut Editor,
        a: &CropCall,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let frame = photo_frame(p, index, a.quadro)?;
        let FrameContent::Photo { crop: now, .. } = &p.spreads[index].frames[frame].content else {
            return Err(ToolError::EmptyFrame {
                spread: a.lamina,
                frame: a.quadro,
            });
        };
        let now = *now;
        let wanted = Crop {
            zoom: a.zoom.unwrap_or(now.zoom),
            center_x: a.centro_x.unwrap_or(now.center_x),
            center_y: a.centro_y.unwrap_or(now.center_y),
        };
        let after = crop_for(p, index, frame, wanted).map_err(bad_crop)?;
        if same(after, now) {
            editor.go_to_spread(index);
            return self.edited(editor, index, "O recorte já estava assim; nada mudou.");
        }
        edit_on(editor, index, "Recortar foto", |p| {
            set_crop(p, index, frame, wanted).map(|_| ())
        })?;
        let text = format!(
            "Quadro {} da lâmina {} recortado: zoom {}, centro ({}; {}).",
            a.quadro,
            a.lamina,
            decimal(after.zoom),
            decimal(after.center_x),
            decimal(after.center_y)
        );
        self.edited(editor, index, &text)
    }
}

/// O mesmo recorte, sem contar o último dígito das contas de ponto flutuante.
fn same(a: Crop, b: Crop) -> bool {
    let near = |x: f64, y: f64| (x - y).abs() < 1e-9;
    near(a.zoom, b.zoom) && near(a.center_x, b.center_x) && near(a.center_y, b.center_y)
}

/// Zoom ou centro fora da faixa viram a recusa da spec; o resto segue como erro do editor.
fn bad_crop(e: EditorError) -> ToolError {
    match e {
        EditorError::Core(CoreError::InvalidCrop(_)) => ToolError::BadCrop,
        other => other.into(),
    }
}
