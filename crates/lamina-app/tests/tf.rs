//! Cenário da spec `tirar-fotos-do-album` (TF) que olha a tela: o botão direito numa foto da grade
//! abre o menu (TF-B10).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

fn ui(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(file);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn tf_b10_botao_direito_na_grade_abre_o_menu() {
    let strip = ui("photo_strip.slint");
    // O TouchArea da miniatura pega o clique direito antes do ContextMenuArea, então é ele que
    // abre o menu, com nome para ser chamado.
    assert!(strip.contains("menu := ContextMenuArea {"));
    let right = strip
        .find("e.button == PointerEventButton.right && e.kind == PointerEventKind.down")
        .unwrap();
    let branch = &strip[right..];
    let show = branch
        .find("menu.show({x: self.mouse-x, y: self.mouse-y});")
        .unwrap();
    let lit = branch.find("if (!root.lit) {").unwrap();
    let lit_end = lit + branch[lit..].find('}').unwrap();
    // O menu abre também numa foto já selecionada: a chamada fica fora do `if (!root.lit)`.
    assert!(
        show > lit_end,
        "menu.show precisa ficar fora do if (!root.lit)"
    );
}
