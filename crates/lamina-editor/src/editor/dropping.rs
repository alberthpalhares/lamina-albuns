//! Fotos soltas numa lâmina (spec IM, regras 7 a 9 e 11): na área de lâmina nova ou entre
//! lâminas viram uma lâmina montada como "Montar lâmina" faria (respeitando a montagem do álbum);
//! fora dos quadros entram na lâmina atual, com o layout refeito; sobre um quadro, como antes.

use lamina_core::{PhotoId, Project};

use super::Editor;
use crate::EditorError;
use crate::edits::{layout_with, spread_photos};
use crate::importing::{ImportReport, ImportSummary, add_imported};

/// Onde as fotos arrastadas foram soltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPlace {
    /// Sobre o quadro `n` da lâmina atual (spec FQ).
    Frame(usize),
    /// Na lâmina atual, fora dos quadros.
    Spread,
    /// No vão `n` da faixa (0 = antes da primeira; o número de lâminas = a área de lâmina nova).
    NewSpread(usize),
}

impl Editor {
    /// Solta `photos` em `place` (uma ação, uma edição; regra 9). `false` se nada mudou.
    pub fn drop_photos(&mut self, place: DropPlace, photos: &[PhotoId]) -> bool {
        match place {
            DropPlace::Frame(frame) => photos.first().is_some_and(|p| self.drop_photo(frame, *p)),
            DropPlace::Spread => self.add_to_spread(photos),
            DropPlace::NewSpread(at) => self.new_spread_with(at, photos),
        }
    }

    /// As fotos que um arrasto leva: a seleção inteira, quando a foto arrastada está nela; senão,
    /// só ela.
    pub fn drag_photos(&self, dragged: PhotoId) -> Vec<PhotoId> {
        if self.selection.contains(&dragged) {
            self.selection.clone()
        } else {
            vec![dragged]
        }
    }

    /// Lâmina nova na posição `at`, montada com `photos`: "Nova lâmina com 5 fotos · lâmina 7".
    fn new_spread_with(&mut self, at: usize, photos: &[PhotoId]) -> bool {
        if photos.is_empty() {
            return false;
        }
        let at = at.min(
            self.session
                .as_ref()
                .map_or(0, crate::Session::spread_count),
        );
        let label = format!(
            "Nova lâmina com {} · lâmina {}",
            count(photos.len()),
            at + 1
        );
        let done = self.change(&label, |p, _| new_spread_at(p, at, photos));
        if done {
            self.selection.clear();
            self.go(at);
        }
        done
    }

    /// As fotos entram na lâmina atual e o layout é refeito com todas: "Acrescentar 2 fotos ·
    /// lâmina 4" (regra 8). Uma foto que já está na lâmina não entra de novo.
    fn add_to_spread(&mut self, photos: &[PhotoId]) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        let current = spread_photos(session.project(), session.current());
        let mut all = current.clone();
        all.extend(photos.iter().filter(|p| !current.contains(p)));
        let added = all.len() - current.len();
        if added == 0 {
            return false;
        }
        let label = format!(
            "Acrescentar {} · lâmina {}",
            count(added),
            session.current() + 1
        );
        let done = self.change(&label, |p, i| layout_with(p, i, &all));
        if done {
            self.selection.clear();
        }
        done
    }

    /// Fotos de outro programa soltas na área de lâmina nova (regra 11): importadas e já numa
    /// lâmina nova na posição `at`, numa edição só (IM-B14).
    pub fn add_photos_as_spread(
        &mut self,
        report: &ImportReport,
        at: usize,
    ) -> Result<ImportSummary, EditorError> {
        let session = self.session_mut().ok_or(EditorError::NoProject)?;
        let fresh = crate::importing::fresh_count(session.project(), report);
        let at = at.min(session.spread_count());
        let room = room_at(session.project(), at);
        // Sem foto nova, nada de lâmina vazia: o mesmo resumo de importar (IM-B13).
        if fresh == 0 {
            return self.add_photos(report);
        }
        if fresh > room {
            // Mais do que cabe numa lâmina: todas entram, sem lâmina nova (spec TR, regra 11).
            let mut summary = self.add_photos(report)?;
            summary.over_capacity = Some(room);
            return Ok(summary);
        }
        let session = self.session_mut().ok_or(EditorError::NoProject)?;
        let known: Vec<PhotoId> = session.project().photos.iter().map(|p| p.id).collect();
        let mut summary = ImportSummary::default();
        let label = format!("Importar fotos numa lâmina nova · lâmina {}", at + 1);
        session.edit(&label, |p| {
            summary = add_imported(p, report);
            let fresh: Vec<PhotoId> = p
                .photos
                .iter()
                .map(|ph| ph.id)
                .filter(|id| !known.contains(id))
                .collect();
            new_spread_at(p, at, &fresh)
        })?;
        self.go(at);
        Ok(summary)
    }
}

/// Quantas fotos cabem numa lâmina nova no vão `at` (as pontas com página simples levam menos).
fn room_at(project: &Project, at: usize) -> usize {
    let pages = project
        .spec
        .edges()
        .active_pages(at, project.spreads.len() + 1);
    lamina_layout::MAX_PER_PAGE * (usize::from(pages.left) + usize::from(pages.right))
}

/// "5 fotos", "1 foto".
fn count(n: usize) -> String {
    if n == 1 {
        "1 foto".to_owned()
    } else {
        format!("{n} fotos")
    }
}

/// Uma lâmina nova na posição `at`, montada com a 1ª opção do gerador para `photos`.
fn new_spread_at(project: &mut Project, at: usize, photos: &[PhotoId]) -> Result<(), EditorError> {
    project.add_spread();
    let at = at.min(project.spreads.len() - 1);
    if let Some(new) = project.spreads.pop() {
        project.spreads.insert(at, new);
    }
    layout_with(project, at, photos)
}
