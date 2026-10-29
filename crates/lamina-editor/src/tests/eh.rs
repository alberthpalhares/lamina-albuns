//! Cenários da spec `erros-humanos` (EH): a frase, o que fazer e o detalhe de cada erro que chega
//! à janela. O EH-B10 (a trava) mora no xtask.

use std::path::PathBuf;

use lamina_core::{CoreError, PhotoId};
use lamina_export::{ExportError, ExportOptions};

use super::common::Base;
use super::eh_samples::samples;
use crate::error_log::LOG_FILE;
use crate::{Editor, EditorError, ErrorLog, Human, export_project, guarded};

/// Palavras de programador que nunca aparecem na frase nem no "o que fazer" (regra 4).
const JARGON: [&str; 21] = [
    "JSON",
    "esquema",
    "versão de formato",
    "os error",
    "PhotoId",
    "SpreadId",
    "FrameId",
    "RelRect",
    "Some(",
    "None",
    "::",
    "`",
    "{",
    "}",
    "panic",
    "unwrap",
    "parse",
    "EOF",
    "UTF-8",
    "SQLite",
    "No such file",
];

#[test]
fn eh_b1_todo_erro_tem_frase_o_que_fazer_e_detalhe() {
    let all = samples();
    // 77 variantes: núcleo 13, fotos 4 (duas de Io), render 6, layout 13, exportação 13,
    // arquivo 4, editor 13, modelos 5 e presets 6.
    assert!(all.len() >= 77, "{} exemplos", all.len());
    for sample in all {
        let m = &sample.message;
        let first = m.title.chars().next().unwrap();
        assert!(first.is_uppercase(), "{}", m.title);
        assert!(m.title.ends_with('.'), "{}", m.title);
        assert!(m.title.chars().count() <= 120, "{}", m.title);
        assert!(!m.what_to_do.trim().is_empty(), "{}", m.title);
        assert!(m.details.starts_with(&sample.technical), "{m:?}");
    }
}

#[test]
fn eh_b2_nenhuma_frase_tem_jargao() {
    for sample in samples() {
        let text = sample.message.line();
        for word in JARGON {
            assert!(!text.contains(word), "\"{word}\" em: {text}");
        }
        let io = text
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == "IO");
        assert!(!io, "IO em: {text}");
    }
}

fn editor_with_album() -> (Base, Editor, PathBuf) {
    let base = Base::with_photos(6, (600, 400));
    let file = base.save(base.project(2), "album/festa.lamina");
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    (base, editor, file)
}

#[test]
fn eh_b3_album_danificado() {
    let (base, mut editor, _) = editor_with_album();
    let broken = base.path("album/quebrado.lamina");
    std::fs::write(&broken, "isto não é um álbum").unwrap();
    let err = editor.open(&broken).unwrap_err();
    let m = err.human();
    assert_eq!(
        m.title,
        "Este arquivo de álbum está danificado ou não é um álbum do Lamina."
    );
    assert!(m.what_to_do.contains("cópia de segurança"), "{m:?}");
    assert!(m.details.starts_with("JSON do projeto inválido"), "{m:?}");
}

#[test]
fn eh_b4_album_de_versao_mais_nova() {
    let (base, mut editor, file) = editor_with_album();
    let text = std::fs::read_to_string(&file).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["schema_version"] = serde_json::json!(99);
    let future = base.path("album/futuro.lamina");
    std::fs::write(&future, json.to_string()).unwrap();
    let err = editor.open(&future).unwrap_err();
    let m = err.human();
    assert!(m.title.contains("versão mais nova do Lamina"), "{m:?}");
    assert!(
        m.what_to_do.contains("Ajuda → Verificar atualizações"),
        "{m:?}"
    );
}

#[test]
fn eh_b5_foto_que_sumiu_ao_exportar() {
    let (base, editor, _) = editor_with_album();
    let project = editor.session().unwrap().project().clone();
    std::fs::remove_file(base.path("fotos/f03.jpg")).unwrap();
    let options = ExportOptions::default();
    let err = export_project(&project, &base.path("saida"), &options, &|_, _| {}).unwrap_err();
    let m = err.human();
    assert!(m.title.contains("\"f03.jpg\""), "{m:?}");
    assert!(
        m.what_to_do.contains("Arquivo → Reencontrar fotos…"),
        "{m:?}"
    );
}

#[test]
fn eh_b6_gravacao_que_falha_ao_exportar() {
    let (base, editor, _) = editor_with_album();
    let project = editor.session().unwrap().project().clone();
    let blocked = base.path("saida");
    std::fs::write(&blocked, "um arquivo no lugar da pasta").unwrap();
    let options = ExportOptions::default();
    let err = export_project(&project, &blocked, &options, &|_, _| {}).unwrap_err();
    let m = err.human();
    assert!(m.title.contains("\"saida\""), "{m:?}");
    assert!(
        !m.title.contains(&base.path("").display().to_string()),
        "{m:?}"
    );
    assert!(m.what_to_do.contains("espaço"), "{m:?}");
    assert!(m.what_to_do.contains("gravada"), "{m:?}");
    assert!(m.details.contains(&blocked.display().to_string()), "{m:?}");
}

#[test]
fn eh_b7_exportacao_interrompida_diz_o_que_ficou() {
    let err = ExportError::Stopped {
        done: vec![
            PathBuf::from("lamina-01.jpg"),
            PathBuf::from("lamina-02.jpg"),
        ],
        source: Box::new(ExportError::Write {
            path: "/saida/lamina-03.jpg".into(),
            reason: "disco cheio".into(),
        }),
    };
    let m = err.human();
    assert_eq!(
        m.title,
        "A exportação parou: não foi possível gravar \"lamina-03.jpg\". 2 arquivos já foram \
         gravados."
    );
    assert!(m.what_to_do.contains("espaço no disco"), "{m:?}");
}

#[test]
fn eh_b8_falha_inesperada_em_segundo_plano_vira_mensagem() {
    let m = guarded(|| -> usize { panic!("índice fora do limite") }).unwrap_err();
    assert_eq!(m.title, "O Lamina teve um problema inesperado.");
    assert!(m.what_to_do.contains("anexe o registro"), "{m:?}");
    assert_eq!(m.details, "índice fora do limite");
    assert_eq!(guarded(|| 2 + 3), Ok(5));
}

#[test]
fn eh_b9_aviso_de_uma_linha_mostra_a_frase_nao_o_texto_tecnico() {
    let (base, mut editor, _) = editor_with_album();
    editor.set_log(ErrorLog::with_home(base.path("registros"), None));
    let ghost: PhotoId = serde_json::from_str("999").unwrap();
    assert!(!editor.drop_photo(0, ghost));
    let technical = EditorError::Core(CoreError::UnknownPhoto(ghost)).to_string();
    let notice = editor.take_notice().unwrap();
    assert!(
        notice.starts_with("Algo não saiu como o esperado"),
        "{notice}"
    );
    assert!(!notice.contains(&technical), "{notice}");
    let log = std::fs::read_to_string(base.path("registros").join(LOG_FILE)).unwrap();
    assert!(log.contains(&format!("detalhe: {technical}")), "{log}");
}

#[test]
fn eh_b11_a_frase_usa_o_nome_e_o_detalhe_usa_o_caminho() {
    let base = Base::with_photos(1, (600, 400));
    let mut request = base.new_album(Some(1));
    request.folder = PathBuf::from(r"C:\Clientes\Ana\Festa");
    let err = crate::create_album(&request, &base.cache(), &|_| {}).unwrap_err();
    let m = err.human();
    assert!(m.title.contains("\"Festa\""), "{m:?}");
    assert!(!m.title.contains("Clientes"), "{m:?}");
    assert!(m.details.contains(r"C:\Clientes\Ana\Festa"), "{m:?}");
}
