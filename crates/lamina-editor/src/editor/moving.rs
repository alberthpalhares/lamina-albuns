//! Fotos entre lâminas (spec ML, regras 1 a 6 e 10): levar a foto de um quadro da lâmina atual
//! para outra lâmina, para um quadro (enche ou troca) ou fora dos quadros (acrescenta), numa
//! edição só com as duas lâminas; e as fotos do navegador soltas numa vizinha. A lâmina atual
//! continua a de origem (escolha 12 da rodada).

use lamina_core::{FrameContent, PhotoId, Project};

use super::Editor;
use crate::edits::{layout_with, place_photo, spread_photos};
use crate::edits_cross::{check_room, move_to_frame, move_to_spread};

/// Onde a foto levada de outra lâmina cai.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossTarget {
    /// Um quadro de foto (vazio ou com foto) da lâmina `spread`.
    Frame { spread: usize, frame: usize },
    /// A lâmina `spread`, fora dos quadros de foto: a foto entra como uma a mais.
    Spread(usize),
}

/// A foto do quadro `frame` da lâmina `spread`, se houver.
fn photo_at(project: &Project, spread: usize, frame: usize) -> Option<PhotoId> {
    match &project.spreads.get(spread)?.frames.get(frame)?.content {
        FrameContent::Photo { photo, .. } => Some(*photo),
        _ => None,
    }
}

/// "Mover foto · lâmina 3 → 5".
fn move_label(from: usize, to: usize) -> String {
    format!("Mover foto · lâmina {} → {}", from + 1, to + 1)
}

impl Editor {
    /// Leva a foto do quadro `from` da lâmina atual para `target`. `false` se nada mudou (o
    /// mesmo quadro, a mesma foto, ou uma recusa, que vira o aviso da janela).
    pub fn move_photo(&mut self, from: usize, target: CrossTarget) -> bool {
        // No modo organizar e no recorte, nada entre lâminas (regra 12; EG, regra 4).
        if self.organizing() || self.crop_mode().is_some() {
            return false;
        }
        let Some(origin) = self.session.as_ref().map(crate::Session::current) else {
            return false;
        };
        match target {
            // Na mesma lâmina, como hoje: sobre um quadro troca; fora dos quadros, nada (regra 6).
            CrossTarget::Frame { spread, frame } if spread == origin => {
                self.drop_frame(from, frame)
            }
            CrossTarget::Spread(spread) if spread == origin => false,
            CrossTarget::Frame { spread, frame } => self.cross_frame(origin, from, (spread, frame)),
            CrossTarget::Spread(spread) => {
                let label = move_label(origin, spread);
                self.change_cross(&label, |p| move_to_spread(p, (origin, from), spread))
            }
        }
    }

    /// Encher (destino vazio) ou trocar (destino com foto) num quadro de outra lâmina.
    fn cross_frame(&mut self, origin: usize, from: usize, (spread, frame): (usize, usize)) -> bool {
        let Some(project) = self.session.as_ref().map(crate::Session::project) else {
            return false;
        };
        let (mine, theirs) = (
            photo_at(project, origin, from),
            photo_at(project, spread, frame),
        );
        // A mesma foto: nada muda e nada entra no histórico (regra 9).
        if mine.is_none() || mine == theirs {
            return false;
        }
        let label = match theirs {
            Some(_) => format!("Trocar fotos · lâminas {} e {}", origin + 1, spread + 1),
            None => move_label(origin, spread),
        };
        self.change_cross(&label, |p| {
            move_to_frame(p, (origin, from), (spread, frame)).map(|_| ())
        })
    }

    /// Uma edição que já diz onde (o rótulo cita as lâminas); a recusa vira o aviso.
    fn change_cross(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Project) -> Result<(), crate::EditorError>,
    ) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        match session.edit(label, f) {
            Ok(()) => true,
            Err(e) => {
                self.report(&e);
                false
            }
        }
    }

    /// Fotos do navegador soltas num quadro de foto de outra lâmina: a primeira vai para ele
    /// (regra 10).
    pub(super) fn place_in(&mut self, (spread, frame): (usize, usize), photo: PhotoId) -> bool {
        let label = format!("Colocar foto · lâmina {}", spread + 1);
        self.change_cross(&label, |p| place_photo(p, spread, frame, photo))
    }

    /// Fotos do navegador soltas numa lâmina fora dos quadros: entram as que ainda não estão
    /// nela, até a capacidade; se nenhuma cabe, o aviso (regras 8 a 10).
    pub(super) fn add_to(&mut self, spread: usize, photos: &[PhotoId]) -> bool {
        let Some(project) = self.session.as_ref().map(crate::Session::project) else {
            return false;
        };
        let present = spread_photos(project, spread);
        let fresh = match check_room(project, spread, &present, photos) {
            Ok(n) => n,
            Err(e) => {
                self.report(&e);
                return false;
            }
        };
        let mut all = present.clone();
        all.extend(photos.iter().filter(|p| !present.contains(p)));
        let label = format!(
            "Acrescentar {} · lâmina {}",
            if fresh == 1 {
                "1 foto".to_owned()
            } else {
                format!("{fresh} fotos")
            },
            spread + 1
        );
        let done = self.change_cross(&label, |p| layout_with(p, spread, &all));
        if done {
            self.selection.clear();
        }
        done
    }
}
