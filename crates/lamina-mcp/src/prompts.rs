//! Os prompts prontos pelo protocolo (spec IA, seção 3): a lista e o texto de cada um, com os
//! argumentos do fotógrafo. Os textos moram no `lamina-editor`.

use lamina_editor::agent::{READY_PROMPTS, prompt_text};
use rmcp::ErrorData;
use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptArgument,
    PromptMessage, Role,
};

pub(crate) fn list() -> ListPromptsResult {
    let prompts = READY_PROMPTS
        .iter()
        .map(|p| {
            let args = p.arguments.iter().map(|(name, description, required)| {
                PromptArgument::new(*name)
                    .with_description(*description)
                    .with_required(*required)
            });
            Prompt::new(p.name, Some(p.description), Some(args.collect())).with_title(p.title)
        })
        .collect();
    ListPromptsResult::with_all_items(prompts)
}

pub(crate) fn get(request: GetPromptRequestParams) -> Result<GetPromptResult, ErrorData> {
    let args = request.arguments.unwrap_or_default();
    let arg = |key: &str| {
        args.get(key)
            .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
    };
    let text = prompt_text(&request.name, &arg).ok_or_else(|| {
        ErrorData::invalid_params(format!("Não existe o prompt {}.", request.name), None)
    })?;
    Ok(GetPromptResult::new(vec![PromptMessage::new_text(
        Role::User,
        text,
    )]))
}
