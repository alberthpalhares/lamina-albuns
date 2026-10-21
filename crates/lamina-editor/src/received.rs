//! Os arquivos recebidos do sistema (spec MA, regra 2): o duplo clique, o "Abrir com", o ícone do
//! Dock e o `lamina-app <arquivo>`. Aqui só se decide o que fazer; a janela executa.

use std::path::{Path, PathBuf};

use crate::Editor;

/// Aviso quando chega mais de um álbum: abre o primeiro, com o nome dele.
pub fn one_at_a_time(name: &str) -> String {
    format!("O Lamina abre um álbum por vez: abriu “{name}”.")
}

/// Aviso quando nada do que chegou é álbum.
pub const NOT_AN_ALBUM: &str = "O Lamina só abre álbuns (arquivos .lamina).";

/// O que a janela faz com os arquivos recebidos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Received {
    /// Abrir este álbum; `ask` = perguntar antes se salva o atual (há alterações).
    Open {
        path: PathBuf,
        ask: bool,
        notice: Option<String>,
    },
    /// O álbum recebido já é o aberto: nada é relido, a janela só fica na frente.
    AlreadyOpen,
    /// Nada do que chegou é álbum: só o aviso.
    Ignore { notice: String },
}

/// É um álbum do Lamina? Só pela extensão, sem diferença de maiúsculas.
pub fn is_album(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("lamina"))
}

/// A decisão: `open` é o arquivo do álbum aberto (se tem), `unsaved` diz se há alterações.
pub fn choose(paths: &[PathBuf], open: Option<&Path>, unsaved: bool) -> Received {
    let mut albums = paths.iter().filter(|p| is_album(p));
    let Some(first) = albums.next() else {
        return Received::Ignore {
            notice: NOT_AN_ALBUM.to_owned(),
        };
    };
    if open.is_some_and(|o| o == first.as_path()) {
        return Received::AlreadyOpen;
    }
    let notice = albums.next().map(|_| {
        let name = first.file_stem().unwrap_or_default().to_string_lossy();
        one_at_a_time(&name)
    });
    Received::Open {
        path: first.clone(),
        ask: unsaved,
        notice,
    }
}

/// O mesmo arquivo por caminhos diferentes (relativo, com `..`) conta como o mesmo. Só para
/// comparar: o caminho aberto continua o recebido.
fn same_file(a: &Path, b: &Path) -> bool {
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    a == b || real(a) == real(b)
}

impl Editor {
    /// A decisão para os arquivos recebidos, com o álbum aberto agora.
    pub fn receive(&self, paths: &[PathBuf]) -> Received {
        let open = self.session().and_then(|s| s.file());
        match choose(paths, open, self.needs_confirmation()) {
            Received::Open { path, .. } if open.is_some_and(|o| same_file(o, &path)) => {
                Received::AlreadyOpen
            }
            other => other,
        }
    }
}
