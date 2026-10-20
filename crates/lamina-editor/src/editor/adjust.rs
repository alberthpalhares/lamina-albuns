//! Os ajustes da foto pelo editor (spec AF, regras 1 e 8, seção 6): endireitar, brilho,
//! contraste, preto e branco e restaurar, nas fotos dos quadros selecionados. Os ajustes moram na
//! foto, então seguem a foto em qualquer quadro. Arrastar um controle ou digitar emenda numa
//! edição só.

use lamina_core::{Crop, FrameContent, PhotoAdjust, PhotoId};

use super::Editor;
use super::crop::set_crop;
use crate::frame_form::parse_number;

/// O ângulo digitado que não é número (seção 6).
pub const BAD_ANGLE: &str = "Use um número de −45 a 45, como 0,3.";
/// A escala digitada que não é número.
pub const BAD_SCALE: &str = "Use um número de 100 a 800, como 150.";

/// Uma mudança nos ajustes da foto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustChange {
    /// O ângulo em décimos de grau (preso a −45° e 45°).
    Angle(i32),
    /// Gira mais `n` décimos de grau (as teclas `[` e `]`).
    Turn(i32),
    Brightness(i32),
    Contrast(i32),
    Bw(bool),
    /// Volta a 0°, 0, 0 e cor.
    Restore,
}

impl AdjustChange {
    /// O rótulo do histórico (regra 8).
    fn label(self) -> &'static str {
        match self {
            AdjustChange::Angle(_) | AdjustChange::Turn(_) => "Endireitar foto",
            AdjustChange::Brightness(_) => "Brilho",
            AdjustChange::Contrast(_) => "Contraste",
            AdjustChange::Bw(_) => "Preto e branco",
            AdjustChange::Restore => "Restaurar ajustes",
        }
    }

    /// Os controles que se arrastam emendam numa edição só.
    fn merges(self) -> bool {
        !matches!(self, AdjustChange::Bw(_) | AdjustChange::Restore)
    }

    fn turns(self) -> bool {
        matches!(self, AdjustChange::Angle(_) | AdjustChange::Turn(_))
    }

    fn apply(self, a: &mut PhotoAdjust) {
        let max = PhotoAdjust::MAX_ANGLE_TENTHS;
        let tone = |v: i32| v.clamp(-PhotoAdjust::MAX_TONE, PhotoAdjust::MAX_TONE);
        match self {
            AdjustChange::Angle(t) => a.angle_tenths = t.clamp(-max, max),
            AdjustChange::Turn(t) => a.angle_tenths = (a.angle_tenths + t).clamp(-max, max),
            AdjustChange::Brightness(v) => a.brightness = tone(v),
            AdjustChange::Contrast(v) => a.contrast = tone(v),
            AdjustChange::Bw(on) => a.bw = on,
            AdjustChange::Restore => *a = PhotoAdjust::default(),
        }
    }
}

/// Quantos décimos de grau cada tecla de endireitar gira (`[` e `]`; com Shift, 1°).
pub(super) fn turn_of(command: crate::Command) -> Option<i32> {
    use crate::Command as C;
    match command {
        C::TurnLeft => Some(-1),
        C::TurnRight => Some(1),
        C::TurnLeftMore => Some(-10),
        C::TurnRightMore => Some(10),
        _ => None,
    }
}

/// As linhas do modo recorte (seção 4): os terços sempre; a grade fina enquanto o ângulo muda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CropGrid {
    pub thirds: bool,
    pub fine: bool,
}

/// "45,3" → 453 décimos; "0,25" → 3 (0,1° de passo, arredondado); texto: erro.
pub fn parse_angle(text: &str) -> Result<i32, &'static str> {
    let text = text.trim().trim_end_matches('°').replace('−', "-");
    let degrees = parse_number(&text).ok_or(BAD_ANGLE)?;
    let max = f64::from(PhotoAdjust::MAX_ANGLE_TENTHS);
    let tenths = (degrees * 10.0).round().clamp(-max, max);
    // Preso entre −450 e 450, então cabe em i32.
    #[allow(clippy::cast_possible_truncation)]
    let tenths = tenths as i32;
    Ok(tenths)
}

impl Editor {
    /// Os quadros de foto selecionados e as fotos deles (cada foto uma vez, na ordem).
    fn photo_targets(&self) -> (Vec<usize>, Vec<PhotoId>) {
        let Some(s) = self.session.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let Some(spread) = s.project().spreads.get(s.current()) else {
            return (Vec::new(), Vec::new());
        };
        let mut frames = Vec::new();
        let mut photos: Vec<PhotoId> = Vec::new();
        for f in self.selected_frames() {
            if let Some(FrameContent::Photo { photo, .. }) =
                spread.frames.get(f).map(|f| &f.content)
            {
                frames.push(f);
                if !photos.contains(photo) {
                    photos.push(*photo);
                }
            }
        }
        (frames, photos)
    }

    /// Os ajustes da foto selecionada (a primeira, com várias).
    pub fn photo_adjust(&self) -> Option<PhotoAdjust> {
        let (_, photos) = self.photo_targets();
        let s = self.session.as_ref()?;
        Some(s.project().photo(*photos.first()?).ok()?.adjust)
    }

    /// Há foto selecionada para ajustar? O ângulo pede um quadro só (AF-B19).
    pub fn can_adjust(&self, change: AdjustChange) -> bool {
        let (frames, photos) = self.photo_targets();
        !photos.is_empty() && (!change.turns() || frames.len() == 1)
    }

    /// Aplica `change` nas fotos selecionadas, como uma edição (ou emendando a anterior).
    pub fn adjust(&mut self, change: AdjustChange) -> bool {
        if !self.can_adjust(change) {
            return false;
        }
        let (_, photos) = self.photo_targets();
        if change.turns() {
            self.turning = true;
        }
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let unchanged = photos.iter().all(|id| {
            s.project().photo(*id).is_ok_and(|p| {
                let mut a = p.adjust;
                change.apply(&mut a);
                a == p.adjust
            })
        });
        if unchanged {
            return false;
        }
        let apply = |p: &mut lamina_core::Project| {
            for photo in p.photos.iter_mut().filter(|ph| photos.contains(&ph.id)) {
                change.apply(&mut photo.adjust);
            }
            Ok::<(), crate::EditorError>(())
        };
        let label = change.label();
        let done = if change.merges() {
            let key = format!("ajuste:{label}:{photos:?}");
            s.edit_merging(label, &key, apply)
        } else {
            s.edit(label, apply)
        };
        done.is_ok()
    }

    /// O campo do ângulo: "0,3" gira 0,3°; fora de −45 a 45, preso; texto, a mensagem.
    pub fn set_angle_text(&mut self, text: &str) -> Result<bool, &'static str> {
        let tenths = parse_angle(text)?;
        Ok(self.adjust(AdjustChange::Angle(tenths)))
    }

    /// A escala do recorte em % (100 = a foto cobrindo o quadro, com o giro já contado), com um
    /// quadro de foto só selecionado.
    pub fn photo_scale(&self) -> Option<f64> {
        let (frames, _) = self.photo_targets();
        let [frame] = frames.as_slice() else {
            return None;
        };
        let s = self.session.as_ref()?;
        let f = s.project().spreads.get(s.current())?.frames.get(*frame)?;
        match &f.content {
            FrameContent::Photo { crop, .. } => Some(crop.zoom * 100.0),
            _ => None,
        }
    }

    /// O campo da escala (100 a 800%): o zoom do recorte por cima da cobertura do giro (AF-B18).
    pub fn set_scale_text(&mut self, text: &str) -> Result<bool, &'static str> {
        let percent = parse_number(text.trim().trim_end_matches('%')).ok_or(BAD_SCALE)?;
        let zoom = (percent / 100.0).clamp(1.0, Crop::MAX_ZOOM);
        let (frames, _) = self.photo_targets();
        let [frame] = frames.as_slice() else {
            return Ok(false);
        };
        let frame = *frame;
        let Some(s) = self.session.as_mut() else {
            return Ok(false);
        };
        let index = s.current();
        let Some(FrameContent::Photo { crop, .. }) = s.project().spreads[index]
            .frames
            .get(frame)
            .map(|f| f.content.clone())
        else {
            return Ok(false);
        };
        if (crop.zoom - zoom).abs() < 1e-9 {
            return Ok(false);
        }
        let wanted = Crop { zoom, ..crop };
        let key = format!("escala:{index}:{frame}");
        Ok(s.edit_merging("Recortar foto", &key, |p| {
            set_crop(p, index, frame, wanted).map(|_| ())
        })
        .is_ok())
    }

    /// Fim de um arrasto no controle: a próxima mudança é outra edição.
    pub fn end_adjust(&mut self) {
        if let Some(s) = self.session.as_mut() {
            s.end_merge();
        }
    }

    /// As linhas do recorte agora (nada fora do modo recorte; nunca vão para a exportação).
    pub fn crop_grid(&self) -> CropGrid {
        CropGrid {
            thirds: self.crop_frame.is_some(),
            fine: self.crop_frame.is_some() && self.turning,
        }
    }

    /// O ângulo parou de mudar (a janela chama 1 s depois): some a grade fina.
    pub fn angle_settled(&mut self) {
        self.turning = false;
    }
}
