//! Os controles deslizantes da spec CD pelo editor: a escala da foto e o zoom da lâmina,
//! proporcionais ao logaritmo, e os botões de girar 90°.

use lamina_core::{Crop, FrameContent, PhotoAdjust};

use super::Editor;
use super::crop::set_crop;
use super::organize_zoom::{MAX_VIEW_ZOOM, ZoomMove, of_factor};
use crate::sliders::{log_position, log_value, wrap_turn};

impl Editor {
    /// A posição do controle de escala (0 a 1), com um quadro de foto só selecionado.
    pub fn scale_position(&self) -> Option<f64> {
        let percent = self.photo_scale()?;
        Some(log_position(percent / 100.0, 1.0, Crop::MAX_ZOOM))
    }

    /// O controle de escala na posição `pos` (regra 1): um arrasto é um passo, "Escala".
    pub fn set_scale_position(&mut self, pos: f64) -> bool {
        self.set_photo_scale(log_value(pos, 1.0, Crop::MAX_ZOOM) * 100.0)
    }

    /// A escala do recorte em % (100 a 800), pelo controle ou pelo campo, emendando o arrasto.
    pub fn set_photo_scale(&mut self, percent: f64) -> bool {
        let zoom = (percent / 100.0).clamp(1.0, Crop::MAX_ZOOM);
        let (frames, _) = self.photo_targets();
        let [frame] = frames.as_slice() else {
            return false;
        };
        let frame = *frame;
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let index = s.current();
        let Some(FrameContent::Photo { crop, .. }) = s
            .project()
            .spreads
            .get(index)
            .and_then(|spread| spread.frames.get(frame))
            .map(|f| f.content.clone())
        else {
            return false;
        };
        if (crop.zoom - zoom).abs() < 1e-9 {
            return false;
        }
        let wanted = Crop { zoom, ..crop };
        let key = format!("escala:{index}:{frame}");
        s.edit_merging("Escala", &key, |p| {
            set_crop(p, index, frame, wanted).map(|_| ())
        })
        .is_ok()
    }

    /// Clique duplo no controle de escala: volta a 100% numa edição própria (regra 4).
    pub fn reset_scale(&mut self) -> bool {
        self.end_adjust();
        let done = self.set_photo_scale(100.0);
        self.end_adjust();
        done
    }

    /// ↺ 90° (`-1`) ou ↻ 90° (`1`), dando a volta em ±180° (regra 2); cada clique é um passo.
    pub fn turn_quarter(&mut self, way: i32) -> bool {
        let Some(adjust) = self.photo_adjust() else {
            return false;
        };
        let target = wrap_turn(adjust.angle_tenths, way.signum() * 900);
        if !self.can_adjust(super::AdjustChange::Angle(target)) {
            return false;
        }
        self.end_adjust();
        let done = self.quarter(target);
        self.end_adjust();
        done
    }

    fn quarter(&mut self, target: i32) -> bool {
        let (_, photos) = self.photo_targets();
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let target = target.clamp(
            -PhotoAdjust::MAX_ANGLE_TENTHS,
            PhotoAdjust::MAX_ANGLE_TENTHS,
        );
        s.edit("Girar 90°", |p| {
            for photo in p.photos.iter_mut().filter(|ph| photos.contains(&ph.id)) {
                photo.adjust.angle_tenths = target;
            }
            Ok::<(), crate::EditorError>(())
        })
        .is_ok()
    }

    /// A posição do controle de zoom da lâmina (0 = Ajustar, 1 = 8×; regra 3).
    pub fn view_zoom_position(&self) -> f64 {
        log_position(self.view_zoom(), 1.0, MAX_VIEW_ZOOM)
    }

    /// O controle de zoom da lâmina na posição `pos`, em volta do centro da área, sem histórico.
    pub fn set_view_zoom_position(&mut self, pos: f64) -> Option<ZoomMove> {
        if !self.organizing() {
            return None;
        }
        let factor = log_value(pos, 1.0, MAX_VIEW_ZOOM);
        self.zoom_to(of_factor(factor), (0.0, 0.0), false)
    }
}
