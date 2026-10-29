//! Fontes do computador de mentira, para os testes (spec FS, seção 7). A "Fonte A" e a "Fonte B"
//! são arquivos embutidos registrados com outro nome. Assim os testes dão o mesmo resultado no
//! Windows e no CI do Linux, que quase não tem fontes.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};

use parley::fontique::Blob;

use super::fonts::family;
use super::installed::{FontReads, STATE, lock, replace_source};
use super::source::{Face, FontSource};

/// Só com o arquivo normal (o da Great Vibes).
pub const FONT_A: &str = "Fonte A";
/// Normal com negrito (a Montserrat variável) e itálico.
pub const FONT_B: &str = "Fonte B";

/// Enquanto existe, as fontes do computador são as do teste, e nenhum outro teste mexe nelas.
/// Ao sair, o computador fica sem fontes (nunca as de verdade, que mudam de máquina para máquina).
#[derive(Debug)]
pub struct TestFonts {
    _lock: MutexGuard<'static, ()>,
}

static LOCK: Mutex<()> = Mutex::new(());

struct TestSource {
    fonts: Vec<(String, Vec<Face>)>,
    /// Nomes a mais, sem arquivo (as listas grandes do FS-B12 e do FS-B15).
    extra: usize,
    gate: Option<Receiver<()>>,
}

impl FontSource for TestSource {
    fn names(&mut self) -> Vec<String> {
        if let Some(gate) = self.gate.take() {
            // Se quem segura o portão desistiu, a leitura segue.
            let _ = gate.recv();
        }
        let mut names: Vec<String> = self.fonts.iter().map(|(n, _)| n.clone()).collect();
        names.extend((1..=self.extra).map(|i| format!("Teste {i:04}")));
        names
    }

    fn faces(&mut self, name: &str) -> Vec<Face> {
        self.fonts
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, faces)| faces.clone())
            .unwrap_or_default()
    }
}

fn face(bytes: &'static [u8], bold: bool, italic: bool) -> Face {
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
    Face {
        data: Blob::new(data),
        index: 0,
        bold,
        italic,
    }
}

/// Os arquivos de uma embutida: o normal e o itálico, se houver.
fn files(name: &str) -> (&'static [u8], Option<&'static [u8]>) {
    family(name).map_or((&[], None), super::fonts::Family::files)
}

/// A "Fonte A" e a "Fonte B".
fn standard() -> Vec<(String, Vec<Face>)> {
    let (vibes, _) = files("Great Vibes");
    let (montserrat, italic) = files("Montserrat");
    let mut b = vec![face(montserrat, true, false)];
    b.extend(italic.map(|i| face(i, false, true)));
    vec![
        (FONT_A.to_owned(), vec![face(vibes, false, false)]),
        (FONT_B.to_owned(), b),
    ]
}

impl TestFonts {
    fn install(fonts: Vec<(String, Vec<Face>)>, extra: usize, gate: Option<Receiver<()>>) -> Self {
        let held = lock(&LOCK);
        replace_source(Box::new(TestSource { fonts, extra, gate }));
        TestFonts { _lock: held }
    }

    /// Um computador sem nenhuma fonte instalada.
    pub fn none() -> Self {
        Self::install(Vec::new(), 0, None)
    }

    /// A "Fonte A" e a "Fonte B" instaladas.
    pub fn standard() -> Self {
        Self::install(standard(), 0, None)
    }

    /// A "Fonte A", a "Fonte B" e mais `extra` nomes ("Teste 0001"…), sem arquivo.
    pub fn many(extra: usize) -> Self {
        Self::install(standard(), extra, None)
    }

    /// Como [`TestFonts::standard`], mas a leitura dos nomes espera um envio pelo canal (FS-B11).
    pub fn gated() -> (Self, Sender<()>) {
        let (release, gate) = channel();
        (Self::install(standard(), 0, Some(gate)), release)
    }

    /// Uma fonte instalada chamada "Inter", com o desenho da Great Vibes (FS-B5).
    pub fn with_inter() -> Self {
        let (vibes, _) = files("Great Vibes");
        let mut fonts = standard();
        fonts.push(("Inter".to_owned(), vec![face(vibes, false, false)]));
        Self::install(fonts, 0, None)
    }

    /// Quantas vezes os nomes e os arquivos foram lidos desde que este teste começou.
    pub fn reads(&self) -> FontReads {
        lock(&STATE).reads
    }
}

impl Drop for TestFonts {
    fn drop(&mut self) {
        replace_source(Box::new(TestSource {
            fonts: Vec::new(),
            extra: 0,
            gate: None,
        }));
    }
}
