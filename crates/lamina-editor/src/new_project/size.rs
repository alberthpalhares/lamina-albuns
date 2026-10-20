//! A etapa 1, Tamanho (spec NP, seção 3 e regras 2 a 5): os campos na unidade escolhida, o
//! tamanho de onde partiu, as margens por borda, o resumo em pixels e o erro da zona segura.

use lamina_core::{AlbumSpec, CoreError};

use super::units::{NOT_A_NUMBER, Unit};
use crate::Preset;
use crate::album::SpecFields;

/// Mensagens da seção 6.
pub const SAFE_TOO_BIG: &str = "A zona segura é maior que a página. Diminua as margens.";
pub const SIZE_INVALID: &str = "A página e o DPI precisam ser maiores que zero.";

/// Os campos de medida da etapa 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeField {
    Width,
    Height,
    Dpi,
    BleedOuter,
    BleedTopBottom,
    BleedSpine,
    SafeOuter,
    SafeTopBottom,
    SafeSpine,
}

/// A etapa 1 como o fotógrafo deixou: as medidas, a unidade e o tamanho de onde partiu.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SizeForm {
    pub fields: SpecFields,
    pub unit: Unit,
    /// A largura digitada é a da lâmina aberta (o dobro da página)?
    pub open_spread: bool,
    /// O preset escolhido por último e se algum campo mudou depois dele (regra 2).
    from: Option<String>,
    edited: bool,
}

impl SizeForm {
    /// O formulário com as medidas de um álbum pronto ("Especificação do álbum…").
    pub fn new(fields: SpecFields) -> Self {
        SizeForm {
            fields,
            ..SizeForm::default()
        }
    }

    /// Escolher um tamanho preenche o Personalizado com os valores dele (regra 2).
    pub fn choose(&mut self, preset: &Preset) {
        self.fields = preset.fields.clone();
        self.from = Some(preset.name.clone());
        self.edited = false;
    }

    /// "Quadrado 30 × 30", "Personalizado (a partir de Quadrado 30 × 30)" ou "Personalizado".
    pub fn label(&self) -> String {
        match (&self.from, self.edited) {
            (Some(name), false) => name.clone(),
            (Some(name), true) => format!("Personalizado (a partir de {name})"),
            (None, _) => "Personalizado".to_owned(),
        }
    }

    /// Troca a unidade: os números da tela mudam, a medida não (regra 3).
    pub fn set_unit(&mut self, unit: Unit) {
        self.unit = unit;
    }

    /// O texto do campo, na unidade escolhida.
    pub fn text(&self, field: SizeField) -> String {
        let f = &self.fields;
        let spread = if self.open_spread { 2.0 } else { 1.0 };
        let mm = match field {
            SizeField::Dpi => return f.dpi.to_string(),
            SizeField::Width => f.width_cm * 10.0 * spread,
            SizeField::Height => f.height_cm * 10.0,
            SizeField::BleedOuter => f.bleed().outer.get(),
            SizeField::BleedTopBottom => f.bleed().top_bottom.get(),
            SizeField::BleedSpine => f.bleed().inner.get(),
            SizeField::SafeOuter => f.safe().outer.get(),
            SizeField::SafeTopBottom => f.safe().top_bottom.get(),
            SizeField::SafeSpine => f.safe().inner.get(),
        };
        self.unit.show(mm)
    }

    /// Guarda o que foi digitado no campo. Texto que não é número fica de fora (seção 6).
    pub fn set(&mut self, field: SizeField, text: &str) -> Result<(), &'static str> {
        if field == SizeField::Dpi {
            self.fields.dpi = text.trim().parse().map_err(|_| NOT_A_NUMBER)?;
        } else {
            let mm = self.unit.read(text)?;
            let spread = if self.open_spread { 2.0 } else { 1.0 };
            let f = &mut self.fields;
            match field {
                SizeField::Width => f.width_cm = mm / 10.0 / spread,
                SizeField::Height => f.height_cm = mm / 10.0,
                SizeField::BleedOuter => f.bleed_mm = mm,
                SizeField::BleedTopBottom => f.bleed_top_mm = Some(mm),
                SizeField::BleedSpine => f.bleed_spine_mm = Some(mm),
                SizeField::SafeOuter => f.safe_mm = mm,
                SizeField::SafeTopBottom => f.safe_top_mm = Some(mm),
                SizeField::SafeSpine | SizeField::Dpi => f.spine_mm = Some(mm),
            }
        }
        self.edited = true;
        Ok(())
    }

    /// "Igual em todas": a sangria e a zona segura de cada borda passam a ser as da externa.
    pub fn same_margins(&mut self) {
        let f = &mut self.fields;
        (f.bleed_top_mm, f.bleed_spine_mm, f.safe_top_mm, f.spine_mm) = (None, None, None, None);
        self.edited = true;
    }

    /// A especificação, ou a mensagem que deixa o Próximo desabilitado.
    pub fn spec(&self) -> Result<AlbumSpec, &'static str> {
        self.fields.to_spec().map_err(|e| match e {
            CoreError::SafeMarginTooLarge { .. } => SAFE_TOO_BIG,
            _ => SIZE_INVALID,
        })
    }

    pub fn error(&self) -> Option<&'static str> {
        self.spec().err()
    }

    /// "Lâmina aberta 60 × 30 cm · 2 páginas de 30 × 30 cm · arquivo de 7157 × 3614 px a 300
    /// DPI", com os pixels da exportação (regra 5).
    pub fn summary(&self) -> Option<String> {
        let spec = self.spec().ok()?;
        let px = lamina_render::print_size(&spec);
        let (w, h) = (spec.page_width().get(), spec.page_height().get());
        let (u, unit) = (self.unit, self.unit.label());
        Some(format!(
            "Lâmina aberta {} × {} {unit} · 2 páginas de {} × {} {unit} · arquivo de {} × {} px \
             a {} DPI",
            u.show(w * 2.0),
            u.show(h),
            u.show(w),
            u.show(h),
            px.width,
            px.height,
            spec.dpi().get()
        ))
    }
}
