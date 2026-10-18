//! Os campos e textos das janelas, decididos aqui e não no `lamina-app` (regra 1; auditoria do
//! Marco 2, M4): os campos do "Novo álbum" e da "Exportar", os passos do painel de aparência, a
//! dica da barra de baixo e os textos do painel de informações.

use std::path::Path;

use lamina_core::{ColorSpace, ExportPrefs, Mm, OutputFormat, PageLimits, Project, Rgb};

use crate::Editor;
use crate::album::SpecFields;
use crate::editor::Look;
use crate::exporting::{DEFAULT_QUALITY, ExportFormat, ExportOptions};
use crate::photos::PhotoInfo;

/// Número com no máximo uma casa e vírgula: 30 → "30", 2,5 → "2,5".
pub fn decimal(v: f64) -> String {
    format!("{}", (v * 10.0).round() / 10.0).replace('.', ",")
}

/// "2,5 mm".
pub fn mm_text(v: f64) -> String {
    format!("{} mm", decimal(v))
}

fn parse_decimal(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok()
}

/// Os campos da janela "Novo álbum" como o fotógrafo digitou (spec JP, seção 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecText {
    pub width_cm: String,
    pub height_cm: String,
    pub dpi: String,
    pub bleed_mm: String,
    pub safe_mm: String,
    /// Vazio = igual à zona segura.
    pub spine_mm: String,
    pub starts_single: bool,
    pub ends_single: bool,
}

impl From<&SpecFields> for SpecText {
    fn from(f: &SpecFields) -> Self {
        SpecText {
            width_cm: decimal(f.width_cm),
            height_cm: decimal(f.height_cm),
            dpi: f.dpi.to_string(),
            bleed_mm: decimal(f.bleed_mm),
            safe_mm: decimal(f.safe_mm),
            spine_mm: f.spine_mm.map(decimal).unwrap_or_default(),
            starts_single: f.starts_single,
            ends_single: f.ends_single,
        }
    }
}

impl SpecText {
    /// Os campos como medidas; `Err` com a mensagem do primeiro campo errado.
    pub fn parse(&self) -> Result<SpecFields, String> {
        let field = |text: &str, name: &str| {
            parse_decimal(text).ok_or_else(|| format!("{name}: digite um número."))
        };
        Ok(SpecFields {
            width_cm: field(&self.width_cm, "Largura")?,
            height_cm: field(&self.height_cm, "Altura")?,
            dpi: self
                .dpi
                .trim()
                .parse()
                .map_err(|_| "DPI: digite um número inteiro.".to_owned())?,
            bleed_mm: field(&self.bleed_mm, "Sangria")?,
            safe_mm: field(&self.safe_mm, "Zona segura")?,
            spine_mm: if self.spine_mm.trim().is_empty() {
                None
            } else {
                Some(field(&self.spine_mm, "Lombada")?)
            },
            starts_single: self.starts_single,
            ends_single: self.ends_single,
        })
    }
}

/// A faixa de páginas como o fotógrafo digitou ("" = sem limite; spec PR).
pub fn parse_limits(min: &str, max: &str) -> Result<PageLimits, String> {
    let field = |text: &str, name: &str| -> Result<Option<u32>, String> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        match text.parse::<u32>() {
            Ok(n) if n > 0 => Ok(Some(n)),
            _ => Err(format!("{name}: digite um número inteiro maior que zero.")),
        }
    };
    let limits = PageLimits {
        min: field(min, "Páginas mínimas")?,
        max: field(max, "Páginas máximas")?,
    };
    match (limits.min, limits.max) {
        (Some(a), Some(b)) if a > b => Err("O mínimo de páginas passa do máximo.".to_owned()),
        _ => Ok(limits),
    }
}

/// Os dois campos da faixa de páginas ("" = sem limite).
pub fn limits_text(limits: PageLimits) -> (String, String) {
    let text = |v: Option<u32>| v.map(|n| n.to_string()).unwrap_or_default();
    (text(limits.min), text(limits.max))
}

/// Posição do formato no campo Formato da janela Exportar (JPEG, PDF, TIFF).
pub fn format_index(format: OutputFormat) -> i32 {
    match format {
        OutputFormat::Jpeg => 0,
        OutputFormat::Pdf => 1,
        OutputFormat::Tiff => 2,
    }
}

/// "Dê um nome ao álbum." quando o nome está vazio.
pub fn album_name_error(name: &str) -> Option<&'static str> {
    name.trim().is_empty().then_some("Dê um nome ao álbum.")
}

/// Os campos da janela "Exportar" (spec EI, seção 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportForm {
    pub quality: i32,
    pub all_spreads: bool,
    pub range: String,
    pub pages: bool,
    /// JPEG, PDF (spec PD) ou TIFF (spec CT).
    pub format: ExportFormat,
    /// Marcas de corte; só valem no PDF.
    pub crop_marks: bool,
    /// sRGB ou Adobe RGB (spec CT).
    pub space: ColorSpace,
}

impl ExportForm {
    /// O que o projeto passa a lembrar desta exportação (spec PR, regra 5).
    pub fn prefs(&self) -> ExportPrefs {
        ExportPrefs {
            format: match self.format {
                ExportFormat::Pdf => OutputFormat::Pdf,
                ExportFormat::Tiff => OutputFormat::Tiff,
                _ => OutputFormat::Jpeg,
            },
            space: self.space,
            pages: self.pages,
        }
    }

    /// As opções da exportação; a qualidade fica entre 1 e 100, e as marcas de corte só valem
    /// no PDF (no JPEG a janela nem mostra a opção).
    pub fn options(&self, overwrite: bool) -> ExportOptions {
        ExportOptions {
            quality: u8::try_from(self.quality.clamp(1, 100)).unwrap_or(DEFAULT_QUALITY),
            range: (!self.all_spreads).then(|| self.range.trim().to_owned()),
            pages: self.pages,
            overwrite,
            format: self.format,
            crop_marks: self.format == ExportFormat::Pdf && self.crop_marks,
            space: self.space,
        }
    }
}

/// A pergunta antes de substituir arquivos que já existem (spec EI, regra 4).
pub fn replace_question(existing: usize, folder: &Path) -> String {
    let files = if existing == 1 {
        "1 arquivo desta exportação".to_owned()
    } else {
        format!("{existing} arquivos desta exportação")
    };
    format!(
        "Já existem {files} em {}. Substituindo, os arquivos lamina-NN.jpg e pagina-NN.jpg que \
         não fizerem parte desta exportação também saem da pasta; os outros arquivos ficam.",
        folder.display()
    )
}

/// O que os botões − e + do painel de aparência mudam (spec AP, seção 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookField {
    Gap,
    Border,
    Radius,
}

impl LookField {
    /// Passo de cada clique e o máximo, em mm.
    pub fn step_and_max(self) -> (f64, f64) {
        match self {
            LookField::Gap => (0.5, 20.0),
            LookField::Border => (0.5, 5.0),
            LookField::Radius => (1.0, 10.0),
        }
    }

    fn value(self, look: &Look) -> f64 {
        match self {
            LookField::Gap => look.gap.get(),
            LookField::Border => look.border.width.get(),
            LookField::Radius => look.radius.get(),
        }
    }
}

/// Borda que uma cor escolhida sem espessura ganha, em mm.
pub const BORDER_FIRST: f64 = 1.0;

impl Editor {
    /// Um clique em − (`delta` < 0) ou + do painel de aparência. Devolve o aviso do editor (o
    /// espaço reduzido para caber), se houver.
    pub fn step_look(&mut self, field: LookField, delta: i32) -> Option<String> {
        let look = self.look()?;
        let (step, max) = field.step_and_max();
        let value = Mm((field.value(&look) + f64::from(delta) * step).clamp(0.0, max));
        match field {
            LookField::Gap => self.set_gap(value),
            LookField::Border => {
                self.set_border(value, look.border.color);
                None
            }
            LookField::Radius => {
                self.set_radius(value);
                None
            }
        }
    }

    /// Escolher a cor da borda: sem espessura ainda, a borda ganha [`BORDER_FIRST`].
    pub fn pick_border_color(&mut self, color: Rgb) -> bool {
        let Some(look) = self.look() else {
            return false;
        };
        let width = if look.border.width.get() > 0.0 {
            look.border.width
        } else {
            Mm(BORDER_FIRST)
        };
        self.set_border(width, color)
    }

    /// A dica da barra de baixo, conforme o que está selecionado.
    pub fn hint(&self) -> &'static str {
        if self.crop_mode().is_some() {
            "Roda do mouse ou + e − para o zoom · arraste para mover · 0 encaixa · Esc sai"
        } else if self.selected_frame().is_some() {
            "Delete tira a foto · C ou duplo clique recorta · arraste para outro quadro para trocar"
        } else {
            ""
        }
    }
}

/// "30 × 30 cm por página" e "300 DPI · sangria 3 mm".
pub fn album_info(project: &Project) -> String {
    let spec = &project.spec;
    format!(
        "{} × {} cm por página\n{} DPI · sangria {} mm",
        decimal(spec.page_width().get() / 10.0),
        decimal(spec.page_height().get() / 10.0),
        spec.dpi().get(),
        decimal(spec.bleed().outer.get())
    )
}

/// Nome, medidas, captura, câmera e quantas vezes a foto foi usada.
pub fn photo_text(info: &PhotoInfo) -> String {
    let uses = match info.uses {
        0 => "Não usada".to_owned(),
        1 => "Usada 1 vez".to_owned(),
        n => format!("Usada {n} vezes"),
    };
    format!(
        "{}\n{} × {} px\nCaptura: {}\nCâmera: {}\n{uses}",
        info.name, info.width, info.height, info.taken_at, info.camera
    )
}
