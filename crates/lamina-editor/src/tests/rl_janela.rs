//! O rolo de lâminas na janela, conferido pelos arquivos das telas (spec RL, regras 1 e 23): a
//! faixa de lâminas é opcional e some sem o cabeçalho; o rolo não cria vagas para as vizinhas
//! (RL-B1 e RL-B23; a parte com o modelo da janela fica nos testes do `lamina-app`).

use super::tl_common::{editor_at, three_spreads};
use crate::{Command, Strip, StripsStore};

const BOTTOM: &str = include_str!("../../../lamina-app/ui/bottom.slint");
const ROLL: &str = include_str!("../../../lamina-app/ui/roll.slint");

#[test]
fn rl_b23_faixa_de_laminas_desligada_some_sem_cabecalho_e_volta_com_alt_1() {
    // Desde a spec FX (regras 10 e 11), a faixa recolhida deixa o cabeçalho, como as outras, e um
    // arquivo antigo a abre uma vez; Alt+1 continua recolhendo e abrindo.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, r#"{ "laminas": false }"#).unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load(&file));
    let open = |e: &crate::Editor| {
        let s = e.strips();
        [Strip::Spreads, Strip::Timeline, Strip::Photos].map(|k| s.is_open(k))
    };
    assert_eq!(open(&editor), [true, false, true]);
    // Lâminas e Sequência são abas (spec DJ, regra 3), e a fileira existe sempre; Fotografias à parte.
    assert!(BOTTOM.contains("FilterTab { text: \"Lâminas\";"));
    assert!(BOTTOM.contains("FilterTab { text: \"Sequência\";"));
    assert!(BOTTOM.contains("name: \"Fotografias\";"));
    assert!(editor.run(Command::ToggleSpreads));
    assert_eq!(open(&editor), [false, false, true]);
    assert!(editor.run(Command::ToggleSpreads));
    assert_eq!(open(&editor), [true, false, true]);
}

#[test]
fn rl_b1_vizinhas_do_rolo_sao_imagens_sem_vagas() {
    // O rolo desenha cada lâmina com as colunas da imagem pronta, nunca com a lâmina viva
    // (`SpreadCanvas`, que tem as vagas).
    assert!(!ROLL.contains("SpreadCanvas"));
    assert!(ROLL.contains("export struct RollCell"));
    let cell = ROLL
        .split("export struct RollCell {")
        .nth(1)
        .and_then(|s| s.split('}').next())
        .unwrap();
    assert!(
        !cell.contains("frames"),
        "uma linha do rolo não tem quadros"
    );
}
