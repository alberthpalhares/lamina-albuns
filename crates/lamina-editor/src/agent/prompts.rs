//! Prompts prontos (spec IA, seção 3): textos fixos do Lamina que o fotógrafo escolhe no cliente
//! de IA. Cada um descreve o passo a passo com as ferramentas; sozinho, não faz nada.

/// Um prompt pronto, com os argumentos que o cliente pede ao fotógrafo.
#[derive(Debug, Clone, Copy)]
pub struct ReadyPrompt {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// (nome, descrição, obrigatório).
    pub arguments: &'static [(&'static str, &'static str, bool)],
}

/// Os quatro prompts prontos.
pub const READY_PROMPTS: [ReadyPrompt; 4] = [
    ReadyPrompt {
        name: "revisar_para_impressao",
        title: "Revisar para impressão",
        description: "Percorre o álbum procurando problemas de impressão e propõe correções.",
        arguments: &[],
    },
    ReadyPrompt {
        name: "montar_album_da_pasta",
        title: "Montar álbum da pasta",
        description: "Cria um álbum a partir de uma pasta de fotos e ajusta lâmina por lâmina.",
        arguments: &[
            ("pasta", "Pasta com as fotos.", true),
            ("laminas", "Número de lâminas (opcional).", false),
        ],
    },
    ReadyPrompt {
        name: "contar_a_historia",
        title: "Contar a história",
        description: "Revê a ordem e os destaques das lâminas, momento a momento.",
        arguments: &[],
    },
    ReadyPrompt {
        name: "legendar",
        title: "Legendar",
        description: "Sugere títulos de capítulo e legendas para as lâminas.",
        arguments: &[],
    },
];

const CLOSING: &str = "Não aplique nada sem a minha aprovação. Cada mudança sua entra no histórico \
                       do Lamina como \"IA: …\" e pode ser desfeita. No fim, pergunte se deve \
                       salvar com salvar_album.";

/// O texto do prompt `name` com os argumentos do fotógrafo; `None` se o prompt não existe.
pub fn prompt_text(name: &str, arg: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let body = match name {
        "revisar_para_impressao" => "Revise o álbum aberto no Lamina para a gráfica. Use \
             resumo_do_album e pre_impressao, depois percorra cada lâmina com ver_lamina. Além dos \
             avisos do Lamina, olhe nas prévias: rosto na dobra ou perto do corte, foto tremida, \
             fotos parecidas em sequência. Faça uma lista por lâmina, com o problema e a correção \
             que você propõe."
            .to_owned(),
        "montar_album_da_pasta" => {
            let folder = arg("pasta").unwrap_or_else(|| "(pergunte a pasta)".to_owned());
            let spreads = arg("laminas").map_or_else(String::new, |n| format!(" com {n} lâminas"));
            format!(
                "Crie um álbum com as fotos de {folder}{spreads} usando novo_album. Depois veja \
                 as miniaturas com ver_fotos, identifique os protagonistas e as melhores fotos de \
                 cada momento, e proponha ajustes lâmina por lâmina (layouts com sugerir_layouts, \
                 trocas de fotos) para as melhores fotos ficarem nos maiores quadros."
            )
        }
        "contar_a_historia" => "Revise a ordem das lâminas do álbum aberto como uma história: \
             abertura forte, momentos na sequência certa e um fechamento. Use listar_fotos, \
             ver_fotos e ver_lamina. Proponha lâminas para mover (mover_lamina), destaques para \
             trocar e layouts que deem mais peso às fotos principais."
            .to_owned(),
        "legendar" => "Sugira um título para cada capítulo do álbum aberto e legendas curtas \
             onde fizer sentido, em português, no tom do álbum. Use ver_lamina para entender cada \
             lâmina. Mostre as sugestões em lista, por lâmina."
            .to_owned(),
        _ => return None,
    };
    Some(format!("{body} {CLOSING}"))
}
