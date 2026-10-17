//! A sessão ao reencontrar fotos (spec EI, regras 5 e 7): caminhos e medidas novos entram como
//! uma edição, e a lista de ausentes e os avisos são refeitos.

use std::convert::Infallible;

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
                }
            }
            Ok(())
        });
        self.missing = missing_of(self.project());
        n
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
