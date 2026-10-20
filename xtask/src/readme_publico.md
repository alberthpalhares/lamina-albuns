# Lamina

O Lamina é um programa gratuito e de código aberto para **diagramar álbuns de fotos**: você
escolhe a pasta das fotos e ele monta o álbum, lâmina por lâmina; depois é trocar os layouts,
pôr títulos e exportar o arquivo para a gráfica (JPEG, TIFF ou PDF de impressão). Funciona no
Windows, no Mac e no Linux. Projeto acadêmico, em fase de testes (versões *beta*).

- [Baixar e instalar](#baixar-e-instalar)
- [Instalar pelo terminal](#instalar-pelo-terminal)
- [Primeiros passos](#primeiros-passos)
- [Sugestões e problemas](#sugest%C3%B5es-e-problemas)
- [Usar com o Claude Desktop](#usar-com-o-claude-desktop)
- [Para quem programa](#para-quem-programa)
- [Licença e créditos](#licen%C3%A7a-e-cr%C3%A9ditos)

## Baixar e instalar

1. Abra a [página de versões](https://github.com/alberthpalhares/lamina-albuns/releases). A
   versão mais nova fica no topo; o selo **Pre-release** quer dizer que é uma versão de teste.
2. O começo das notas da versão tem o link do arquivo do seu sistema:

   | Sistema | Arquivo |
   |---|---|
   | Windows 10 ou 11 | o que termina em `Setup.exe` |
   | Mac (Intel ou Apple) | o que termina em `Setup.pkg` |
   | Linux | o que termina em `.AppImage` |

3. Na primeira vez, o Windows e o Mac avisam que não reconhecem o programa, porque o Lamina ainda
   não tem a assinatura paga da Microsoft e da Apple. O aviso não quer dizer vírus. O
   [guia de instalação](crates/lamina-app/dist/instalar.md) mostra cada clique, nos três
   sistemas.

Depois de instalado, o Lamina avisa sozinho quando há versão nova e se atualiza com um clique.

## Instalar pelo terminal

Para quem prefere: estes comandos instalam o Lamina na sua pasta de usuário, sem pedir senha de
administrador e sem o aviso do sistema (o aviso só não aparece; o antivírus continua examinando
os arquivos). Enquanto não houver versão estável, eles instalam a versão de teste mais nova.

**Windows** (no PowerShell):

```powershell
irm https://raw.githubusercontent.com/alberthpalhares/lamina-albuns/main/crates/lamina-app/dist/instalar.ps1 | iex
```

**Mac e Linux** (no Terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/alberthpalhares/lamina-albuns/main/crates/lamina-app/dist/instalar.sh | sh
```

Para sempre ficar com a versão de teste mais nova: rode `$env:LAMINA_BETA = 1` antes do comando
no Windows, ou troque o fim por `| sh -s -- --beta` no Mac e no Linux. No Mac, o Lamina fica em
`Aplicativos` da sua pasta pessoal; no Linux, em `~/.local/bin/Lamina.AppImage` (abra uma vez
por ele para o atalho entrar no menu).

## Primeiros passos

Na tela de boas-vindas, **Experimentar com o álbum de exemplo** abre um álbum pronto, com fotos
livres, para você ver o Lamina funcionando. Depois, **Novo álbum com minhas fotos…** monta o seu.

## Sugestões e problemas

No Lamina, **Ajuda → Enviar sugestão ou problema…** abre um formulário curto, sem precisar de
conta, já com a versão do programa. Se algo deu errado, **Ajuda → Abrir pasta de registros**
mostra o arquivo `lamina.log` para anexar. O Lamina não envia nada sozinho. Quem tem conta no
GitHub também pode abrir uma [issue](https://github.com/alberthpalhares/lamina-albuns/issues).

## Usar com o Claude Desktop

Na página da versão, baixe `Lamina-Windows.mcpb` ou `Lamina-macOS.mcpb` e dê dois cliques: o
Claude Desktop instala o Lamina e pergunta as pastas dos álbuns. **Com "Permitir que a IA veja
as fotos" ligada, as prévias das lâminas e as miniaturas das fotos vão para o provedor da IA**
(no Claude, a Anthropic). São fotos de clientes (LGPD): desligue a opção se o contrato pedir; a
IA continua só com os dados das fotos e das lâminas.

## Para quem programa

O Lamina é escrito em Rust, com a interface em Slint.

```sh
cargo run --release -p lamina-app   # a janela
cargo xtask verify                  # formatação, lint, testes e checagens
```

No Linux, a janela precisa de `libfontconfig1-dev`, `libxkbcommon-dev` e `libwayland-dev`.

## Licença e créditos

- Código: Apache-2.0 (arquivo [`LICENSE`](LICENSE)).
- Interface: [Slint](https://slint.dev), sob a licença Royalty-free 2.0.
- Fontes em `crates/lamina-render/fonts/`: SIL OFL 1.1 (o `OFL.txt` fica na pasta de cada uma).
- Fotos do álbum de exemplo: CC0 1.0, de Syced no Wikimedia Commons
  ([créditos](crates/lamina-app/exemplo/CREDITOS.md)).
