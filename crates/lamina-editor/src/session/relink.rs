//! A sessão ao reencontrar fotos (spec EI, regras 5 e 7): caminhos e medidas novos entram como
//! uma edição, e a lista de ausentes e os avisos são refeitos.

use std::convert::Infallible;

use lamina_core::PhotoId;

use super::{Session, missing_of};
use crate::relink::Found;

impl Session {
    /// Aplica as fotos achadas como a edição "Reencontrar fotos" (o projeto fica com alterações
    /// não salvas). Devolve quantas fotos mudaram; 0 = nada mudou.
    pub fn relink(&mut self, found: &[Found]) -> usize {
        let found: Vec<&Found> = found
            .iter()
            .filter(|f| self.missing.contains(&f.id))
            .collect();
        let n = found.len();
        if n == 0 {
            return 0;
        }
        let what = if n == 1 {
            "1 foto".to_owned()
        } else {
            format!("{n} fotos")
        };
        let label = format!("Reencontrar fotos · {what}");
        let Ok(()) = self.edit::<Infallible>(&label, |p| {
            for f in &found {
                if let Some(photo) = p.photos.iter_mut().find(|photo| photo.id == f.id) {
                    photo.path.clone_from(&f.path);
                    photo.width_px = f.width;
                    photo.height_px = f.height;
                    if let Some(print) = &f.fingerprint {
                        photo.fingerprint = Some(print.clone());
                    }
                }
            }
            Ok(())
        });
        self.missing = missing_of(self.project());
        n
    }

    /// A janela "Algumas fotos mudaram de lugar" abre uma vez por abertura, e só quando ainda
    /// faltam fotos depois da procura de sempre (spec RF, regra 2). `true` = abrir agora.
    pub fn take_relink_prompt(&mut self) -> bool {
        if self.missing.is_empty() || self.relink_asked {
            return false;
        }
        self.relink_asked = true;
        true
    }

    /// O resultado de conferir de novo (spec RF, regra 6): as fotos que ainda faltam nos mesmos
    /// caminhos. Não é uma edição. `true` = alguma voltou.
    pub fn checked_missing(&mut self, still: &[PhotoId]) -> bool {
        let now: Vec<PhotoId> = self
            .missing
            .iter()
            .copied()
            .filter(|id| still.contains(id))
            .collect();
        let changed = now.len() != self.missing.len();
        self.missing = now;
        changed
    }

    /// A procura ao abrir chega depois, em segundo plano: se o fotógrafo já desfez algo, aplicar
    /// agora apagaria o "refazer" dele. Aí nada muda e a janela sugere "Procurar fotos
    /// ausentes…" (auditoria do Marco 2, M8). `None` = não aplicado.
    pub fn relink_after_open(&mut self, found: &[Found]) -> Option<usize> {
        if self.can_redo() {
            return None;
        }
        Some(self.relink(found))
    }
}
