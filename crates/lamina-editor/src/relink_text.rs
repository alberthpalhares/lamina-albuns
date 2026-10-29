//! Os textos da janela "Algumas fotos mudaram de lugar" (spec RF, seção 4): o que aconteceu,
//! cada pasta e o que fazer, em palavras de fotógrafo.

use std::path::Path;

use crate::relink_folders::{MissingFolder, missing_folders};
use crate::{Editor, Session};

pub const TITLE: &str = "Algumas fotos mudaram de lugar";
pub const INTRO: &str = "O Lamina guarda onde cada foto estava. Estas pastas não estão mais lá:";
pub const HINT: &str = "Se você moveu a pasta (para um SSD externo, por exemplo), mostre onde ela \
                        está agora. As fotos dela são achadas sozinhas.";
pub const PLUG_IN: &str = "Se as fotos estão num disco externo, conecte o disco e clique em \
                           Tentar de novo.";
/// A pergunta de "Localizar esta foto…" com um arquivo de outro conteúdo (regra 4).
pub const DIFFERENT: &str = "Este arquivo não é a mesma foto: o conteúdo é diferente. Usar mesmo \
                             assim?";
/// Quantas pastas a janela mostra antes de "e mais N" (regra 1).
pub const FOLDERS_SHOWN: usize = 5;

/// O que a janela mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelinkView {
    pub folders: Vec<MissingFolder>,
    /// "e mais 3 pastas", quando há mais de [`FOLDERS_SHOWN`].
    pub more: Option<String>,
    /// Alguma pasta está num disco desconectado: a janela mostra [`PLUG_IN`].
    pub unplugged: bool,
}

/// A janela para a sessão de agora; `None` quando não falta foto.
pub fn relink_view(session: &Session) -> Option<RelinkView> {
    let mut folders = missing_folders(session.project(), session.missing());
    if folders.is_empty() {
        return None;
    }
    let unplugged = folders.iter().any(|f| f.disk.is_some());
    let extra = folders.len().saturating_sub(FOLDERS_SHOWN);
    folders.truncate(FOLDERS_SHOWN);
    let more = match extra {
        0 => None,
        1 => Some("e mais 1 pasta".to_owned()),
        n => Some(format!("e mais {n} pastas")),
    };
    Some(RelinkView {
        folders,
        more,
        unplugged,
    })
}

/// "Nenhuma das 248 fotos de “Casamento” foi encontrada em E:\Fotos." (seção 6).
pub fn not_found_in(folder: &MissingFolder, place: &Path) -> String {
    let name = folder.name();
    if folder.count == 1 {
        format!(
            "A foto de “{name}” não foi encontrada em {}.",
            place.display()
        )
    } else {
        format!(
            "Nenhuma das {} fotos de “{name}” foi encontrada em {}.",
            folder.count,
            place.display()
        )
    }
}

impl Editor {
    /// A foto do quadro `frame` da lâmina atual, se ela é uma foto ausente (regra 8: o duplo
    /// clique e o botão direito levam a "Localizar esta foto…").
    pub fn missing_photo_at(&self, frame: usize) -> Option<lamina_core::PhotoId> {
        let s = self.session()?;
        let spread = s.project().spreads.get(s.current())?;
        match spread.frames.get(frame)?.content {
            lamina_core::FrameContent::Photo { photo, .. } if s.missing().contains(&photo) => {
                Some(photo)
            }
            _ => None,
        }
    }
}
