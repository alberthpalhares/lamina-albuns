//! Modelos do fotógrafo pelo editor (spec MD): o arquivo dos modelos, salvar a lâmina atual,
//! renomear e apagar levando junto as marcas de favorito e rejeitado.

use lamina_layout::{SpreadModel, model_key};

use super::Editor;
use crate::models::{ModelError, ModelsStore, model_from_spread};

impl Editor {
    pub fn models(&self) -> &[SpreadModel] {
        self.models.all()
    }

    /// Usa os modelos deste arquivo; o aviso de leitura, se houver, vira aviso da janela.
    pub fn set_models(&mut self, mut store: ModelsStore) {
        if let Some(warning) = store.warning.take() {
            self.notice = Some(warning);
        }
        self.models = store;
        self.lists.borrow_mut().clear();
    }

    pub fn model_exists(&self, name: &str) -> bool {
        self.models.exists(name)
    }

    /// A lâmina atual pode virar modelo? (tem quadro de foto; regra 1)
    pub fn can_save_model(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| model_from_spread(s.project(), s.current(), "x").is_ok())
    }

    /// Salva a lâmina atual como o modelo `name` (substitui um de mesmo nome).
    pub fn save_model(&mut self, name: &str) -> Result<(), ModelError> {
        let s = self.session.as_ref().ok_or(ModelError::NoPhotoFrame)?;
        let model = model_from_spread(s.project(), s.current(), name)?;
        self.models.save(model)?;
        self.lists.borrow_mut().clear();
        Ok(())
    }

    /// Renomeia o modelo; as marcas de favorito e rejeitado vão junto (regra 5).
    pub fn rename_model(&mut self, old: &str, new: &str) -> Result<bool, ModelError> {
        if !self.models.rename(old, new)? {
            return Ok(false);
        }
        let prefs = &mut self.prefs.prefs;
        for mirrored in [false, true] {
            let (from, to) = (model_key(old, mirrored), model_key(new.trim(), mirrored));
            for set in [&mut prefs.favorites, &mut prefs.rejected] {
                if set.remove(&from) {
                    set.insert(to.clone());
                }
            }
        }
        self.after_models_change();
        Ok(true)
    }

    /// Apaga o modelo e as marcas dele.
    pub fn remove_model(&mut self, name: &str) -> Result<bool, ModelError> {
        if !self.models.remove(name)? {
            return Ok(false);
        }
        let prefs = &mut self.prefs.prefs;
        for mirrored in [false, true] {
            let key = model_key(name, mirrored);
            prefs.favorites.remove(&key);
            prefs.rejected.remove(&key);
        }
        self.after_models_change();
        Ok(true)
    }

    fn after_models_change(&mut self) {
        self.lists.borrow_mut().clear();
        if let Err(message) = self.prefs.save() {
            self.notice = Some(message);
        }
    }
}
