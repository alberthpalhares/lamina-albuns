# Lâmina

Diagramador de álbuns fotográficos em Rust, para Windows, macOS e Linux. Projeto acadêmico.

## Compilar

```bash
cargo run --release -p lamina-app   # a janela
cargo xtask verify                  # formatação, lint, testes e checagens
```

No Linux, a janela precisa de `libfontconfig1-dev`, `libxkbcommon-dev` e `libwayland-dev`.

## Licença

Apache-2.0 (arquivo `LICENSE`). A interface usa o Slint sob a licença Royalty-free 2.0. As fontes em `crates/lamina-render/fonts/` têm a licença SIL OFL 1.1 (o `OFL.txt` fica na pasta de cada uma).
