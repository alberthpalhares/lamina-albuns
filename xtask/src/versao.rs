//! `cargo xtask versao <tag> [--notas <arquivo>]` (spec DS, regra 11): a release só segue se a
//! tag é a versão do `Cargo.toml` e as novidades (`crates/lamina-app/dist/novidades.md`) têm a
//! seção dela. Com `--notas`, grava a seção num arquivo, para as notas da versão. As novidades
//! ficam junto do código porque a release roda no repositório público, que não tem o CHANGELOG.

use lamina_editor::updates::news;

/// As notas de cada versão, em linguagem simples.
pub const NOTES_FILE: &str = "crates/lamina-app/dist/novidades.md";

/// O repositório público, num lugar só (spec VP, regra 7).
macro_rules! repo {
    () => {
        "alberthpalhares/lamina-app"
    };
}
pub const REPO: &str = repo!();

/// O guia de instalação para leigos (spec PC, regra 7), publicado no repositório público.
pub const GUIDE_URL: &str = concat!(
    "https://github.com/",
    repo!(),
    "/blob/main/crates/lamina-app/dist/instalar.md"
);

/// Onde a release guarda os arquivos (o repositório público).
const DOWNLOAD: &str = concat!("https://github.com/", repo!(), "/releases/download");

/// As notas de qualquer versão começam com o arquivo de cada sistema, com link direto, e terminam
/// com o guia e o caminho do formulário (spec PC, regra 9; auditoria da Fase 4, achados 1 e 3).
pub fn with_guide(notes: &str, tag: &str) -> String {
    let channel = if tag.contains('-') { "-beta" } else { "" };
    let link = |name: String| format!("[`{name}`]({DOWNLOAD}/{tag}/{name})");
    format!(
        "**Para instalar, baixe o arquivo do seu sistema:** Windows → {} · Mac → {} · Linux → {}. \
         Passo a passo, com os avisos do sistema: {GUIDE_URL}\n\n{notes}\n\n**Primeira vez?** Veja \
         como instalar: {GUIDE_URL}\n\nAchou um problema ou tem uma ideia? No Lamina, use Ajuda → \
         Enviar sugestão ou problema.\n",
        link(format!("Lamina-win{channel}-Setup.exe")),
        link(format!("Lamina-osx{channel}-Setup.pkg")),
        link(format!("Lamina-linux{channel}.AppImage")),
    )
}

/// A versão só sai com o endereço de verdade do formulário de sugestões (spec SR, seção 8).
pub fn form_ready(form: &str) -> Result<(), String> {
    if form.contains("a-definir") {
        Err(format!(
            "o formulário de sugestões ainda não tem endereço ({form}): ponha o link do Tally em \
             FEEDBACK_FORM (lamina-editor/src/feedback.rs)"
        ))
    } else {
        Ok(())
    }
}

/// A versão de `[workspace.package]` no `Cargo.toml`.
pub fn workspace_version(cargo_toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(rest) = line.strip_prefix("version") {
            let value = rest.trim_start().strip_prefix('=')?.trim();
            return Some(value.trim_matches('"').to_owned());
        }
    }
    None
}

/// O texto da seção `## [<versão>]` das novidades, pelo mesmo leitor do Lamina (spec NV, regra 8).
pub fn notes(news: &str, version: &str) -> Option<String> {
    news::section_text(news, version)
}

/// O aviso, sem reprovar, quando um item da seção que vai sair não tem "Experimente:" (spec NV,
/// regra 9).
pub fn try_warning(news: &str, version: &str) -> Option<String> {
    let section = news::parse(news)
        .into_iter()
        .find(|s| s.version.to_string() == version)?;
    news::missing_try(&section)
}

/// Confere a tag (`v0.2.0` ou `v0.2.0-beta.1`). `Ok` traz as notas da versão.
pub fn check(tag: &str, cargo_toml: &str, news: &str) -> Result<String, String> {
    let Some(version) = tag.strip_prefix('v') else {
        return Err(format!(
            "a tag {tag} precisa começar com \"v\" (ex.: v0.2.0)"
        ));
    };
    let Some(cargo) = workspace_version(cargo_toml) else {
        return Err("o Cargo.toml não tem a versão em [workspace.package]".to_owned());
    };
    if cargo != version {
        return Err(format!(
            "a tag é {version}, mas o Cargo.toml está na {cargo}: mude um dos dois"
        ));
    }
    notes(news, version).ok_or_else(|| {
        format!("o {NOTES_FILE} não tem a seção \"## [{version}]\" com as novidades da versão")
    })
}

pub fn run(args: &[String]) -> bool {
    let (tag, out) = match args {
        [tag] => (tag, None),
        [tag, flag, file] if flag == "--notas" => (tag, Some(file)),
        _ => {
            eprintln!("uso: cargo xtask versao <tag> [--notas <arquivo>]");
            return false;
        }
    };
    let read = |p: &str| std::fs::read_to_string(p).unwrap_or_default();
    let news = read(NOTES_FILE);
    if let Some(warning) = try_warning(&news, tag.trim_start_matches('v')) {
        eprintln!("  ! versao: {warning}");
    }
    let checked = form_ready(lamina_editor::FEEDBACK_FORM)
        .and_then(|()| check(tag, &read("Cargo.toml"), &news));
    match checked {
        Ok(text) => {
            if let Some(file) = out
                && let Err(e) = std::fs::write(
                    file,
                    with_guide(&crate::vitrine::note_images(&text, tag), tag),
                )
            {
                eprintln!("  ✘ versao: não deu para gravar {file}: {e}");
                return false;
            }
            println!("  versao ok ({tag})");
            true
        }
        Err(e) => {
            eprintln!("  ✘ versao: {e}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARGO: &str = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.2.0\"\nedition = \"2024\"\n\n[package]\nversion = \"9.9.9\"\n";
    const LOG: &str = "# Changelog\n\n## [Não lançado]\n\n- algo\n\n## [0.2.0] - 2026-10-20\n\n- Atualização automática.\n- Instalar pelo terminal.\n\n## [0.1.0]\n\n- Início.\n";

    #[test]
    fn ds_b16_trava_da_versao() {
        assert_eq!(
            check("v0.2.0", CARGO, LOG).unwrap(),
            "- Atualização automática.\n- Instalar pelo terminal."
        );
        let other = check("v0.2.1", CARGO, LOG).unwrap_err();
        assert!(other.contains("Cargo.toml está na 0.2.0"), "{other}");
        let no_section = check("v0.2.0", CARGO, "# Changelog\n\n## [0.1.0]\n\n- Início.\n");
        assert!(
            no_section
                .unwrap_err()
                .contains("não tem a seção \"## [0.2.0]\""),
            "sem a seção"
        );
        let no_v = check("0.2.0", CARGO, LOG).unwrap_err();
        assert!(no_v.contains("começar com \"v\""), "{no_v}");
    }

    #[test]
    fn versao_de_teste_e_secao_vazia() {
        let beta = CARGO.replace("\"0.2.0\"", "\"0.3.0-beta.1\"");
        let log = "## [0.3.0-beta.1]\n\n- Teste.\n";
        assert_eq!(check("v0.3.0-beta.1", &beta, log).unwrap(), "- Teste.");
        assert_eq!(notes("## [0.3.0]\n\n## [0.2.0]\n- x\n", "0.3.0"), None);
    }

    #[test]
    fn pc_b7_as_notas_de_toda_versao_terminam_com_o_guia() {
        let text = with_guide("- Teste.", "v0.1.0-beta.4");
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let tail = &lines[lines.len() - 2..];
        assert_eq!(
            tail[0],
            format!("**Primeira vez?** Veja como instalar: {GUIDE_URL}")
        );
        assert!(
            tail[1].ends_with("use Ajuda → Enviar sugestão ou problema."),
            "{}",
            tail[1]
        );
        assert!(text.contains("\n\n- Teste.\n\n"), "{text}");
    }

    /// Auditoria da Fase 4, achado 1: a primeira linha diz qual arquivo baixar, com link direto.
    #[test]
    fn as_notas_comecam_com_o_arquivo_de_cada_sistema() {
        let beta = with_guide("- Teste.", "v0.1.0-beta.4");
        let first = beta.lines().next().unwrap();
        assert!(first.starts_with("**Para instalar"), "{first}");
        let win = "[`Lamina-win-beta-Setup.exe`](https://github.com/alberthpalhares/lamina-app/\
                   releases/download/v0.1.0-beta.4/Lamina-win-beta-Setup.exe)";
        assert!(first.contains(win), "{first}");
        assert!(first.contains("Lamina-osx-beta-Setup.pkg"), "{first}");
        assert!(first.contains("Lamina-linux-beta.AppImage"), "{first}");
        let stable = with_guide("- Teste.", "v0.2.0");
        assert!(stable.contains("Lamina-win-Setup.exe"), "{stable}");
    }

    #[test]
    fn nv_b7_o_xtask_usa_o_leitor_do_lamina() {
        assert_eq!(
            notes(LOG, "0.2.0").as_deref(),
            Some("- Atualização automática.\n- Instalar pelo terminal.")
        );
        let real =
            std::fs::read_to_string(format!("{}/../{NOTES_FILE}", env!("CARGO_MANIFEST_DIR")))
                .unwrap();
        for section in news::parse(&real) {
            let version = section.version.to_string();
            assert_eq!(notes(&real, &version), news::section_text(&real, &version));
            assert!(notes(&real, &version).is_some(), "{version}");
        }
    }

    #[test]
    fn nv_b11_a_release_avisa_quando_falta_o_experimente() {
        let text = concat!(
            "## [0.1.0-beta.5] - 2026-10-24\n\n",
            "- **Atalhos.** A lista. Experimente: aperte F1.\n",
            "- **Programa menor.** O instalador ficou mais leve.\n",
            "- **Novidades.** A janela. Experimente: abra a Ajuda.\n",
        );
        assert_eq!(
            try_warning(text, "0.1.0-beta.5").as_deref(),
            Some("1 item da 0.1.0-beta.5 sem “Experimente:”: «Programa menor.»")
        );
        let full = text.replace("mais leve.", "mais leve. Experimente: baixe.");
        assert_eq!(try_warning(&full, "0.1.0-beta.5"), None);
        // O aviso não reprova: a seção continua saindo nas notas.
        let beta = CARGO.replace("\"0.2.0\"", "\"0.1.0-beta.5\"");
        assert!(check("v0.1.0-beta.5", &beta, text).is_ok());
    }

    #[test]
    fn versao_nao_sai_sem_o_formulario() {
        let err = form_ready("https://tally.so/r/a-definir").unwrap_err();
        assert!(err.contains("FEEDBACK_FORM"), "{err}");
        assert_eq!(form_ready("https://tally.so/r/abc123"), Ok(()));
    }
}
