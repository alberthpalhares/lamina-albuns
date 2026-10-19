# Novidades do Lamina

As notas de cada versão, em linguagem simples. A release copia daqui a seção da versão
(`cargo xtask versao`). Versão nova entra no topo.

## [0.1.0-beta.3] - 2026-10-19

- **O Lamina no Claude Desktop.** Baixe o `Lamina-Windows.mcpb` (ou o `Lamina-macOS.mcpb`) desta
  página e dê dois cliques: o Claude passa a abrir os seus álbuns, apontar os problemas de
  impressão, trocar layouts, fotos, recortes, textos e a aparência, e exportar para a gráfica.
  Você pede em português comum. Cada mudança dele aparece como "IA: …" e pode ser desfeita, e o
  álbum só é salvo quando você pede.
- **Suas fotos e a LGPD.** Com "Permitir que a IA veja as fotos" ligada, as prévias e as
  miniaturas das fotos dos seus clientes vão para o provedor da IA (a Anthropic, no Claude).
  Desligue essa opção, nas configurações da extensão, se o contrato com o cliente pedir: o Claude
  continua trabalhando só com os dados das fotos. Mais detalhes na seção "Usar com o Claude
  Desktop" do README do Lamina.
- O pacote ainda não tem assinatura digital: o Claude Desktop pode mostrar um aviso ao instalar.
- No Mac, o pacote ainda não foi testado num computador de verdade. Se der problema, conte para
  a gente.

## [0.1.0-beta.2] - 2026-10-19

- A tela Sobre mostra só o Lamina.
- Versão para conferir a atualização automática a partir da 0.1.0-beta.1.

## [0.1.0-beta.1] - 2026-10-19

Primeira versão de teste do Lamina, para os testadores.

- Álbum automático a partir de uma pasta de fotos, com troca de layouts, recorte, texto, modelos
  e presets de impressão.
- Exportação em JPEG, TIFF e PDF/X-4 para a gráfica, com a conferência antes de exportar.
- Instalador para Windows, Mac e Linux, e instalação pelo terminal sem o aviso de segurança.
- Atualização automática: o Lamina avisa quando há versão nova e se atualiza sozinho.
- Duplo clique num álbum `.lamina` abre o Lamina (Windows e Linux).
