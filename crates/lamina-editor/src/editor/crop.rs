//! Recorte pelo editor (spec RC): o modo recorte de um quadro, o recorte provisório durante o
//! gesto e a edição que grava o recorte no fim dele.

use std::borrow::Cow;

use lamina_core::{Crop, FrameContent, Project};

use super::Editor;

/// Passo do zoom: cada passo da roda ou cada + e − (regra 3).
pub const ZOOM_STEP: f64 = 1.1;

impl Editor {
    /// O quadro em modo recorte, se houver.
    pub fn crop_mode(&self) -> Option<usize> {
        self.crop_frame
    }

    /// Foto e recorte do quadro `frame` da lâmina atual (o provisório, se houver).
    fn frame_crop(&self, frame: usize) -> Option<(f64, f64, f64, Crop)> {
        let s = self.session.as_ref()?;
        let project = s.project();
        let f = project.spreads.get(s.current())?.frames.get(frame)?;
        let FrameContent::Photo { photo, crop } = &f.content else {
            return None;
        };
        let p = project.photo(*photo).ok()?;
        let rect = project.frame_rect(f);
        let aspect = rect.width.get() / rect.height.get();
        let crop = self
            .pending_crop
            .filter(|_| self.crop_frame == Some(frame))
            .unwrap_or(*crop);
        Some((f64::from(p.width_px), f64::from(p.height_px), aspect, crop))
    }

    /// Entra no modo recorte do quadro (só quadro com foto; regra 1).
    pub fn enter_crop(&mut self, frame: usize) -> bool {
        self.commit_crop();
        if self.frame_crop(frame).is_none() {
            return false;
        }
        self.selected_frame = Some(frame);
        self.crop_frame = Some(frame);
        true
    }

    /// Sai do modo recorte, gravando o provisório.
    pub fn exit_crop(&mut self) {
        self.commit_crop();
        self.crop_frame = None;
    }

    /// Esc no meio do gesto: descarta o provisório (regra 4).
    pub fn cancel_crop(&mut self) {
        self.pending_crop = None;
    }

    /// Arrasto de `dx`, `dy` (frações do quadro) desde o último movimento: provisório (regra 2).
    pub fn crop_drag(&mut self, dx: f64, dy: f64) -> bool {
        let Some((w, h, aspect, crop)) = self.crop_frame.and_then(|f| self.frame_crop(f)) else {
            return false;
        };
        self.pending_crop = Some(crop.panned(w, h, aspect, dx, dy));
        true
    }

    /// Zoom multiplicado por `factor`, provisório até [`Editor::commit_crop`] (regra 3).
    pub fn crop_zoom(&mut self, factor: f64) -> bool {
        let Some((w, h, aspect, crop)) = self.crop_frame.and_then(|f| self.frame_crop(f)) else {
            return false;
        };
        self.pending_crop = Some(crop.zoomed(w, h, aspect, factor));
        true
    }

    /// Recorte padrão (zoom 1, centro), provisório.
    pub fn crop_reset(&mut self) -> bool {
        if self.crop_frame.is_none() {
            return false;
        }
        self.pending_crop = Some(Crop::default());
        true
    }

    /// Fim do gesto: o provisório vira a edição "Recortar foto" (regra 4). `true` = gravou.
    pub fn commit_crop(&mut self) -> bool {
        let (Some(frame), Some(crop)) = (self.crop_frame, self.pending_crop.take()) else {
            return false;
        };
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        let unchanged = session.project().spreads[index]
            .frames
            .get(frame)
            .is_some_and(
                |f| matches!(&f.content, FrameContent::Photo { crop: c, .. } if *c == crop),
            );
        if unchanged {
            return false;
        }
        session
            .edit("Recortar foto", |p| {
                if let Some(FrameContent::Photo { crop: c, .. }) = p.spreads[index]
                    .frames
                    .get_mut(frame)
                    .map(|f| &mut f.content)
                {
                    *c = crop;
                }
                Ok::<(), lamina_core::CoreError>(())
            })
            .is_ok()
    }

    /// O projeto como a janela mostra: com o recorte provisório aplicado.
    pub fn view_project(&self) -> Option<Cow<'_, Project>> {
        let s = self.session.as_ref()?;
        let (Some(frame), Some(crop)) = (self.crop_frame, self.pending_crop) else {
            return Some(Cow::Borrowed(s.project()));
        };
        let mut project = s.project().clone();
        if let Some(FrameContent::Photo { crop: c, .. }) = project.spreads[s.current()]
            .frames
            .get_mut(frame)
            .map(|f| &mut f.content)
        {
            *c = crop;
        }
        Some(Cow::Owned(project))
    }
}
