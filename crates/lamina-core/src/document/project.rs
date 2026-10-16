//! O projeto inteiro e o formato JSON versionado.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Frame, FrameContent, FrameId, Photo, PhotoId, Rgb, Spread, SpreadId, migration};
use crate::{AlbumSpec, CoreError, Rect, RelRect};

/// Versão atual do formato do arquivo de projeto. Suba ao mudar a estrutura e
/// escreva a migração correspondente em [`Project::from_json`].
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// O projeto inteiro: o que é salvo em disco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub spec: AlbumSpec,
    pub photos: Vec<Photo>,
    pub spreads: Vec<Spread>,
    /// Próximo identificador livre (compartilhado por fotos, lâminas e quadros).
    next_id: u64,
}

impl Project {
    pub fn new(name: impl Into<String>, spec: AlbumSpec) -> Self {
        Project {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: name.into(),
            spec,
            photos: Vec::new(),
            spreads: Vec::new(),
            next_id: 1,
        }
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_photo(
        &mut self,
        path: impl Into<PathBuf>,
        width_px: u32,
        height_px: u32,
    ) -> PhotoId {
        let id = PhotoId(self.take_id());
        self.photos.push(Photo {
            id,
            path: path.into(),
            width_px,
            height_px,
            fingerprint: None,
            taken_at: None,
            camera: None,
        });
        id
    }

    pub fn add_spread(&mut self) -> SpreadId {
        let id = SpreadId(self.take_id());
        self.spreads.push(Spread {
            id,
            background: Rgb::WHITE,
            frames: Vec::new(),
        });
        id
    }

    pub fn add_frame(
        &mut self,
        spread: SpreadId,
        rect: RelRect,
        content: FrameContent,
    ) -> Result<FrameId, CoreError> {
        self.check_content(&content)?;
        let id = FrameId(self.take_id());
        let spread = self.spread_mut(spread)?;
        spread.frames.push(Frame { id, rect, content });
        Ok(id)
    }

    pub fn photo(&self, id: PhotoId) -> Result<&Photo, CoreError> {
        self.photos
            .iter()
            .find(|p| p.id == id)
            .ok_or(CoreError::UnknownPhoto(id))
    }

    pub fn spread(&self, id: SpreadId) -> Result<&Spread, CoreError> {
        self.spreads
            .iter()
            .find(|s| s.id == id)
            .ok_or(CoreError::UnknownSpread(id))
    }

    pub fn spread_mut(&mut self, id: SpreadId) -> Result<&mut Spread, CoreError> {
        self.spreads
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or(CoreError::UnknownSpread(id))
    }

    /// Posição absoluta de um quadro no canvas da lâmina, em mm.
    pub fn frame_rect(&self, frame: &Frame) -> Rect {
        frame.rect.to_absolute(&self.spec.trim_rect())
    }

    /// Resolução efetiva da foto impressa no quadro, em DPI.
    ///
    /// É a base do aviso de resolução: abaixo do DPI da especificação, a foto está sendo
    /// ampliada. Retorna `None` se o quadro não tem foto.
    pub fn effective_dpi(&self, frame: &Frame) -> Result<Option<f64>, CoreError> {
        let FrameContent::Photo { photo, crop } = &frame.content else {
            return Ok(None);
        };
        let photo = self.photo(*photo)?;
        let rect = self.frame_rect(frame);
        // Com zoom 1 a foto cobre o quadro: vale o eixo que precisa de mais ampliação.
        let dpi_x = f64::from(photo.width_px) / rect.width.to_inches();
        let dpi_y = f64::from(photo.height_px) / rect.height.to_inches();
        Ok(Some(dpi_x.min(dpi_y) / crop.zoom))
    }

    fn check_content(&self, content: &FrameContent) -> Result<(), CoreError> {
        if let FrameContent::Photo { photo, crop } = content {
            self.photo(*photo)?;
            crop.validate()?;
        }
        Ok(())
    }

    /// Confere todas as invariantes. Chamado ao carregar um arquivo.
    pub fn validate(&self) -> Result<(), CoreError> {
        self.spec.validate()?;
        for spread in &self.spreads {
            for frame in &spread.frames {
                frame.rect.validate()?;
                self.check_content(&frame.content)?;
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, CoreError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Lê um projeto salvo. Versões antigas do formato são migradas; versões mais novas que
    /// este programa são recusadas.
    pub fn from_json(json: &str) -> Result<Self, CoreError> {
        #[derive(Deserialize)]
        struct VersionOnly {
            schema_version: u32,
        }
        let value: serde_json::Value = serde_json::from_str(json)?;
        let VersionOnly { schema_version } = serde_json::from_value(value.clone())?;
        if schema_version > CURRENT_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchema {
                found: schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let value = migration::migrate(value, schema_version)?;
        let project: Project = serde_json::from_value(value)?;
        project.validate()?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Crop, Dpi, Mm};

    fn sample_project() -> (Project, SpreadId, PhotoId) {
        let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap()
            .with_bleed(Mm(3.0))
            .unwrap();
        let mut project = Project::new("Casamento Ana & Bruno", spec);
        let photo = project.add_photo("fotos/IMG_0001.jpg", 6000, 4000);
        let spread = project.add_spread();
        project
            .add_frame(
                spread,
                RelRect::new(0.0, 0.0, 0.5, 1.0).unwrap(),
                FrameContent::Photo {
                    photo,
                    crop: Crop::default(),
                },
            )
            .unwrap();
        (project, spread, photo)
    }

    #[test]
    fn ida_e_volta_em_json() {
        let (project, _, _) = sample_project();
        let json = project.to_json().unwrap();
        let loaded = Project::from_json(&json).unwrap();
        assert_eq!(loaded, project);
    }

    /// Defeito achado pelo Auto Build (AB-B6): sem `float_roundtrip`, o serde_json lia
    /// 0.21321367521367526 de volta como 0.2132136752136753, e o projeto aberto ficava diferente
    /// do salvo.
    #[test]
    fn ida_e_volta_em_json_preserva_numeros_exatos() {
        let (mut project, spread, photo) = sample_project();
        let rect = RelRect::new(0.213_213_675_213_675_26, 0.1, 0.3, 0.4).unwrap();
        let content = FrameContent::Photo {
            photo,
            crop: Crop::default(),
        };
        project.add_frame(spread, rect, content).unwrap();
        let loaded = Project::from_json(&project.to_json().unwrap()).unwrap();
        assert_eq!(loaded, project);
    }

    #[test]
    fn quadro_relativo_vira_posicao_absoluta() {
        let (project, spread, _) = sample_project();
        let frame = &project.spread(spread).unwrap().frames[0];
        assert_eq!(
            project.frame_rect(frame),
            Rect::new(Mm(3.0), Mm(3.0), Mm(300.0), Mm(300.0))
        );
    }

    #[test]
    fn resolucao_efetiva_de_foto_6000x4000_numa_pagina_de_30cm() {
        let (project, spread, _) = sample_project();
        let frame = &project.spread(spread).unwrap().frames[0];
        // 4000 px de altura em 300 mm (11,81 pol) ≈ 338,7 DPI; o eixo vertical é o limitante.
        let dpi = project.effective_dpi(frame).unwrap().unwrap();
        assert!((dpi - 338.67).abs() < 0.01, "dpi = {dpi}");
    }

    #[test]
    fn zoom_reduz_a_resolucao_efetiva() {
        let (mut project, spread, photo) = sample_project();
        let frame_id = project
            .add_frame(
                spread,
                RelRect::new(0.5, 0.0, 0.5, 1.0).unwrap(),
                FrameContent::Photo {
                    photo,
                    crop: Crop {
                        zoom: 2.0,
                        ..Crop::default()
                    },
                },
            )
            .unwrap();
        let frame = project
            .spread(spread)
            .unwrap()
            .frames
            .iter()
            .find(|f| f.id == frame_id)
            .unwrap();
        let dpi = project.effective_dpi(frame).unwrap().unwrap();
        assert!((dpi - 169.33).abs() < 0.01, "dpi = {dpi}");
    }

    #[test]
    fn quadro_com_foto_inexistente_e_rejeitado() {
        let (mut project, spread, _) = sample_project();
        let result = project.add_frame(
            spread,
            RelRect::full(),
            FrameContent::Photo {
                photo: PhotoId(999),
                crop: Crop::default(),
            },
        );
        assert!(matches!(result, Err(CoreError::UnknownPhoto(_))));
    }

    #[test]
    fn versao_de_formato_mais_nova_e_recusada() {
        let (project, _, _) = sample_project();
        let json = project.to_json().unwrap().replace(
            &format!("\"schema_version\": {CURRENT_SCHEMA_VERSION}"),
            "\"schema_version\": 999",
        );
        assert!(matches!(
            Project::from_json(&json),
            Err(CoreError::UnsupportedSchema { found: 999, .. })
        ));
    }

    #[test]
    fn json_com_quadro_invalido_e_recusado() {
        let (project, _, _) = sample_project();
        let json = project
            .to_json()
            .unwrap()
            .replace("\"width\": 0.5", "\"width\": 1.5");
        assert!(matches!(
            Project::from_json(&json),
            Err(CoreError::InvalidRelRect(_))
        ));
    }
}
