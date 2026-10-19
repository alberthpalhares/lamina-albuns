# Lamina

Diagramador de álbuns fotográficos em Rust, para Windows, macOS e Linux. Projeto acadêmico.

## Instalar

Baixe o instalador do seu sistema na [página de versões](https://github.com/alberthpalhares/lamina-codigo/releases)
(`Lamina-win-Setup.exe`, `Lamina-osx-Setup.pkg` ou `Lamina.AppImage`). Depois de instalado, o
Lamina avisa sozinho quando há versão nova.

Pelo terminal, sem o aviso de segurança do Windows e do Mac (o Lamina não é assinado; o aviso
só não aparece, e o antivírus continua examinando):

| Sistema | Comando |
|---|---|
| Windows (PowerShell) | `irm https://raw.githubusercontent.com/alberthpalhares/lamina-codigo/main/crates/lamina-app/dist/instalar.ps1 \| iex` |
| Mac e Linux | `curl -fsSL https://github.com/alberthpalhares/lamina-codigo/releases/latest/download/instalar.sh \| sh` |

Para a versão de teste mais nova: `$env:LAMINA_BETA = 1` antes do comando no Windows, ou
`sh -s -- --beta` no lugar de `sh` no Mac e no Linux.

## Compilar

```bash
cargo run --release -p lamina-app   # a janela
cargo xtask verify                  # formatação, lint, testes e checagens
```

No Linux, a janela precisa de `libfontconfig1-dev`, `libxkbcommon-dev` e `libwayland-dev`.

## Licença

Apache-2.0 (arquivo `LICENSE`). A interface usa o Slint sob a licença Royalty-free 2.0. As fontes em `crates/lamina-render/fonts/` têm a licença SIL OFL 1.1 (o `OFL.txt` fica na pasta de cada uma).

[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png)](https://slint.dev)
