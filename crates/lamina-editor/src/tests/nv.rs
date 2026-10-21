//! Cenários da spec `o-que-ha-de-novo` (NV): o leitor das notas, quais versões a janela mostra e
//! a abertura que lembra a versão. O NV-B7 e o NV-B11 também têm teste no `xtask` (`versao`).

use super::tl_common::{editor_at, three_spreads};
use crate::updates::news::{
    NO_NEWS, NewsSection, after_update, date_text, from_help, heading, missing_try, parse,
    section_text, since, until, without_try,
};
use crate::updates::{Job, UPDATES_UNREADABLE, Updates, UpdatesStore, Version};
use crate::{Command, Editor};

/// As notas de verdade, as mesmas que vão dentro do Lamina.
const REAL: &str = include_str!("../../../lamina-app/dist/novidades.md");

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

fn versions(sections: &[NewsSection]) -> Vec<String> {
    sections.iter().map(|s| s.version.to_string()).collect()
}

/// Uma seção por versão, com um item cada.
fn notes(list: &[&str]) -> String {
    list.iter()
        .map(|v| format!("## [{v}] - 2026-10-20\n\n- **Item da {v}.** Texto. Experimente: algo.\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn installed(dir: &std::path::Path, last: Option<&str>, current: &str) -> Updates {
    let path = dir.join("atualizacoes.json");
    if let Some(last) = last {
        std::fs::write(
            &path,
            format!(r#"{{"automatico": false, "ultima_versao": "{last}"}}"#),
        )
        .unwrap();
    }
    Updates::new(UpdatesStore::load(&path), v(current), true)
}

fn saved(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("atualizacoes.json")).unwrap_or_default()
}

#[test]
fn nv_b1_o_leitor_separa_versoes_titulos_e_o_experimente() {
    let text = "# Novidades do Lamina\n\nFrase antes das seções.\n\n\
                ## [0.1.0-beta.5] - 2026-10-24\n\nFrase de abertura.\n\n\
                - **Atalhos.** Ajuda → `Atalhos do teclado…` mostra tudo.\n  Experimente: aperte F1.\n\
                - Programa menor.\n\
                - Veja o [link](https://exemplo.org) novo.\n\n\
                ## [0.1.0-beta.4] - 2026-10-20\n\n- Um item.\n";
    let sections = parse(text);
    assert_eq!(versions(&sections), ["0.1.0-beta.5", "0.1.0-beta.4"]);
    let s = &sections[0];
    assert_eq!(s.date.as_deref(), Some("2026-10-24"));
    assert_eq!(s.intro, "Frase de abertura.");
    assert_eq!(s.items.len(), 3);
    let first = &s.items[0];
    assert_eq!(first.title.as_deref(), Some("Atalhos."));
    assert_eq!(first.text, "Ajuda → Atalhos do teclado… mostra tudo.");
    assert_eq!(first.try_it.as_deref(), Some("aperte F1."));
    assert_eq!(
        (s.items[1].title.as_ref(), s.items[1].try_it.as_ref()),
        (None, None)
    );
    assert_eq!(s.items[1].text, "Programa menor.");
    assert_eq!(s.items[2].text, "Veja o link novo.");
    let all = format!("{sections:?}");
    assert!(!all.contains("Frase antes"), "{all}");
}

#[test]
fn nv_b2_todas_as_versoes_desde_a_do_fotografo() {
    let sections = parse(&notes(&[
        "0.1.0-beta.1",
        "0.1.0-beta.2",
        "0.1.0-beta.3",
        "0.1.0-beta.4",
        "0.1.0-beta.5",
        "0.1.0-beta.6",
        "0.2.0-beta.1",
        "0.2.0",
    ]));
    assert_eq!(
        versions(&since(&sections, &v("0.1.0-beta.2"), &v("0.1.0-beta.5"))),
        ["0.1.0-beta.5", "0.1.0-beta.4", "0.1.0-beta.3"]
    );
    assert_eq!(
        versions(&since(&sections, &v("0.1.0-beta.4"), &v("0.1.0-beta.6"))),
        ["0.1.0-beta.6", "0.1.0-beta.5"]
    );
    assert_eq!(
        versions(&since(&sections, &v("0.1.0-beta.6"), &v("0.2.0"))),
        ["0.2.0"],
        "na estável, sem as betas"
    );
}

#[test]
fn nv_b3_aparece_uma_vez_na_primeira_abertura() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = installed(dir.path(), Some("0.1.0-beta.4"), "0.1.0-beta.5");
    assert_eq!(first.on_start().news_since, Some(v("0.1.0-beta.4")));
    assert!(saved(dir.path()).contains("\"ultima_versao\": \"0.1.0-beta.5\""));
    let mut again = installed(dir.path(), None, "0.1.0-beta.5");
    assert_eq!(again.on_start().news_since, None);
}

#[test]
fn nv_b4_instalacao_nova_nao_mostra_nada() {
    let dir = tempfile::tempdir().unwrap();
    let mut fresh = installed(dir.path(), None, "0.1.0-beta.5");
    assert_eq!(fresh.on_start().news_since, None);
    assert!(saved(dir.path()).contains("\"ultima_versao\": \"0.1.0-beta.5\""));
    let broken = tempfile::tempdir().unwrap();
    let path = broken.path().join("atualizacoes.json");
    std::fs::write(&path, "{ilegível").unwrap();
    let mut u = Updates::new(UpdatesStore::load(&path), v("0.1.0-beta.5"), true);
    assert_eq!(u.on_start().news_since, None);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ilegível");
    let notice = u.on_open().notice.unwrap_or_default();
    assert!(notice.contains(UPDATES_UNREADABLE), "{notice}");
}

#[test]
fn nv_b5_sem_notas_versao_menor_ou_compilado_nada_abre() {
    let only_old = parse(&notes(&["0.1.0-beta.3", "0.1.0-beta.4"]));
    // (a) a beta.5 sem notas: nada a mostrar, e a versão fica lembrada.
    let dir = tempfile::tempdir().unwrap();
    let mut a = installed(dir.path(), Some("0.1.0-beta.4"), "0.1.0-beta.5");
    let since_a = a.on_start().news_since.unwrap();
    assert!(since(&only_old, &since_a, &v("0.1.0-beta.5")).is_empty());
    assert_eq!(after_update(&only_old, &since_a, &v("0.1.0-beta.5")), None);
    assert!(saved(dir.path()).contains("0.1.0-beta.5"));
    // (b) voltou de versão.
    let dir = tempfile::tempdir().unwrap();
    let mut b = installed(dir.path(), Some("0.1.0-beta.5"), "0.1.0-beta.4");
    assert_eq!(b.on_start().news_since, None);
    assert!(saved(dir.path()).contains("\"ultima_versao\": \"0.1.0-beta.4\""));
    // (c) o Lamina compilado pela equipe não abre nem grava.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("atualizacoes.json");
    std::fs::write(&path, r#"{"ultima_versao": "0.1.0-beta.4"}"#).unwrap();
    let mut c = Updates::new(UpdatesStore::load(&path), v("0.1.0-beta.5"), false);
    assert_eq!(c.on_start().news_since, None);
    c.on_open();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        r#"{"ultima_versao": "0.1.0-beta.4"}"#
    );
    // LAMINA_NOVIDADES simula, sem gravar nada.
    assert_eq!(
        c.simulate_start("0.1.0-beta.4").news_since,
        Some(v("0.1.0-beta.4"))
    );
    assert_eq!(c.simulate_start("não é versão").news_since, None);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        r#"{"ultima_versao": "0.1.0-beta.4"}"#
    );
}

#[test]
fn nv_b6_pela_ajuda_todas_a_atual_primeiro() {
    let sections = parse(&notes(&[
        "0.1.0-beta.1",
        "0.1.0-beta.2",
        "0.1.0-beta.3",
        "0.1.0-beta.4",
        "0.1.0-beta.5",
        "0.1.0-beta.6",
    ]));
    let page = from_help(&sections, &v("0.1.0-beta.5"));
    assert_eq!(page.title, "O que há de novo no Lamina");
    assert_eq!(
        versions(&page.sections),
        [
            "0.1.0-beta.5",
            "0.1.0-beta.4",
            "0.1.0-beta.3",
            "0.1.0-beta.2",
            "0.1.0-beta.1"
        ]
    );
    assert_eq!(
        page.current,
        Some(v("0.1.0-beta.5")),
        "a beta.5 é \"Esta versão\""
    );
    assert_eq!(until(&sections, &v("0.1.0-beta.5")), page.sections);
    assert!(page.empty.is_empty());
    assert_eq!(
        from_help(&parse("# Novidades\n"), &v("0.1.0-beta.5")).empty,
        NO_NEWS
    );
    assert_eq!(NO_NEWS, "Ainda não há notas desta versão.");
    // O comando: disponível sem álbum, com álbum e no modo organizar, e não muda nada.
    assert!(Editor::new().available(Command::ShowNews));
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(0));
    let history = editor.session().unwrap().history_list();
    assert!(editor.available(Command::ShowNews));
    assert!(!editor.run(Command::ShowNews));
    assert_eq!(editor.selected_frame(), Some(0));
    assert!(editor.run(Command::Organize));
    assert!(editor.available(Command::ShowNews));
    assert!(!editor.run(Command::ShowNews));
    assert!(editor.organizing());
    assert_eq!(editor.session().unwrap().history_list(), history);
    editor.set_busy(true);
    assert!(
        !editor.available(Command::ShowNews),
        "durante uma exportação"
    );
}

#[test]
fn nv_b7_um_leitor_so_para_o_lamina_e_para_a_release() {
    let log = "# Changelog\n\n## [Não lançado]\n\n- algo\n\n## [0.2.0] - 2026-10-20\n\n- Atualização automática.\n- Instalar pelo terminal.\n\n## [0.1.0]\n\n- Início.\n";
    assert_eq!(
        section_text(log, "0.2.0").as_deref(),
        Some("- Atualização automática.\n- Instalar pelo terminal.")
    );
    assert_eq!(
        section_text("## [0.3.0]\n\n## [0.2.0]\n- x\n", "0.3.0"),
        None
    );
    let sections = parse(REAL);
    assert!(sections.len() >= 4, "as betas 1 a 4");
    for s in &sections {
        assert!(!s.items.is_empty(), "{} sem itens", s.version);
        let body = section_text(REAL, &s.version.to_string()).unwrap();
        let date = s
            .date
            .as_ref()
            .map(|d| format!(" - {d}"))
            .unwrap_or_default();
        let again = parse(&format!("## [{}]{date}\n\n{body}\n", s.version));
        assert_eq!(again, std::slice::from_ref(s), "{}", s.version);
    }
}

#[test]
fn nv_b8_cabecalhos_que_nao_sao_versao_e_arquivo_vazio() {
    let text = "## [Não lançado]\n\n- Um.\n- Dois.\n\n## [0.2.0]\n\n- Três.\n\n## Outras notas\n\n- Quatro.\n";
    let sections = parse(text);
    assert_eq!(versions(&sections), ["0.2.0"]);
    let texts: Vec<&str> = sections[0].items.iter().map(|i| i.text.as_str()).collect();
    assert_eq!(texts, ["Três."]);
    assert!(parse("").is_empty());
    assert!(parse("# Novidades\n\nSó texto.\n\n## Outra coisa\n- x\n").is_empty());
}

#[test]
fn nv_b9_substitui_o_aviso_lamina_atualizado_para() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("atualizacoes.json");
    std::fs::write(
        &path,
        r#"{"automatico": true, "ultima_versao": "0.1.0-beta.4"}"#,
    )
    .unwrap();
    let mut u = Updates::new(UpdatesStore::load(&path), v("0.1.0-beta.5"), true);
    let start = u.on_start();
    let open = u.on_open();
    for notice in [&start.notice, &open.notice].into_iter().flatten() {
        assert!(!notice.contains("Lamina atualizado para"), "{notice}");
    }
    assert!(matches!(open.job, Some(Job::Check { manual: false, .. })));
    assert_eq!(u.on_open().job, None, "uma vez só");
    // Com o arquivo ilegível, o aviso de preferências continua (DS-B11).
    std::fs::write(&path, "{").unwrap();
    let mut broken = Updates::new(UpdatesStore::load(&path), v("0.1.0-beta.5"), true);
    broken.on_start();
    assert!(
        broken
            .on_open()
            .notice
            .unwrap_or_default()
            .contains(UPDATES_UNREADABLE)
    );
    // A janela leva a frase no título.
    let sections = parse(&notes(&["0.1.0-beta.5"]));
    let page = after_update(&sections, &v("0.1.0-beta.4"), &v("0.1.0-beta.5")).unwrap();
    assert_eq!(page.title, "Lamina atualizado para 0.1.0-beta.5");
    assert_eq!(
        page.subtitle,
        "Veja o que mudou desde a 0.1.0-beta.4, a versão que você tinha, e experimente:"
    );
}

#[test]
fn nv_b10_mesma_entrada_mesmas_novidades() {
    let text = "## [0.1.0-beta.3] - fim de outubro\n\n- Três.\n\n\
                ## [0.1.0-beta.5] - 2026-10-24\n\n- Cinco.\n\n\
                ## [0.1.0-beta.4]\n\n- Primeira.\n\n\
                ## [0.1.0-beta.4]\n\n- Segunda.\n";
    let (a, b) = (parse(text), parse(text));
    assert_eq!(a, b);
    assert_eq!(
        versions(&a),
        ["0.1.0-beta.5", "0.1.0-beta.4", "0.1.0-beta.3"]
    );
    let four: Vec<&str> = a[1].items.iter().map(|i| i.text.as_str()).collect();
    assert_eq!(four, ["Primeira."]);
    assert_eq!(date_text("2026-10-24"), "24/10/2026");
    assert_eq!(date_text("fim de outubro"), "fim de outubro");
    assert_eq!(heading(&a[0]), "Versão 0.1.0-beta.5 · 24/10/2026");
    assert_eq!(heading(&a[2]), "Versão 0.1.0-beta.3 · fim de outubro");
    assert_eq!(heading(&a[1]), "Versão 0.1.0-beta.4");
}

#[test]
fn nv_b11_a_release_avisa_quando_falta_o_experimente() {
    let text = "## [0.1.0-beta.5] - 2026-10-24\n\n\
                - **Atalhos.** A lista. Experimente: aperte F1.\n\
                - **Programa menor.** O instalador ficou mais leve.\n\
                - Novidades depois de atualizar. **Experimente:** abra a Ajuda.\n";
    let sections = parse(text);
    let missing = without_try(&sections[0]);
    assert_eq!(missing.len(), 1);
    assert_eq!(
        missing_try(&sections[0]).as_deref(),
        Some("1 item da 0.1.0-beta.5 sem “Experimente:”: «Programa menor.»")
    );
    let full = text.replace("mais leve.", "mais leve. Experimente: baixe.");
    assert_eq!(missing_try(&parse(&full)[0]), None);
    let untitled = "## [0.2.0]\n\n- Um texto sem título e sem a dica no fim.\n- Dois.\n";
    assert_eq!(
        missing_try(&parse(untitled)[0]).as_deref(),
        Some("2 itens da 0.2.0 sem “Experimente:”: «Um texto sem título e…», «Dois.»")
    );
}
