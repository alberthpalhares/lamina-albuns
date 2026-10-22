# Novidades do Lamina

As notas de cada versão, em linguagem simples. O Lamina mostra estas notas em "O que há de novo"
depois de atualizar, e a release copia daqui a seção da versão (`cargo xtask versao`). Versão nova
entra no topo, como `## [0.1.0-beta.5] - 2026-10-24`; o que ainda não saiu fica em
`## [Não lançado]`, que o Lamina não mostra.

Cada item diz o que mudou e como ver a mudança, em linguagem de fotógrafo:
`- **O que mudou.** Explicação curta. Experimente: o que fazer para ver.` O `cargo xtask versao`
avisa (sem impedir a versão) quando um item não tem "Experimente:". No fim do item, um comentário
que a página da versão não mostra põe a ilustração (um arquivo de `novidades/`) e o comando do
botão "Experimente agora": `<!-- imagem: atalhos.png; comando: ajuda.atalhos -->`.

## [Não lançado]

## [0.1.0-beta.5] - 2026-10-22

Esta versão põe as lâminas lado a lado, traz a faixa de layouts, o painel de fotos em grade e o álbum que começa em branco, e corrige o Lamina que fechava no Mac ao importar muitas fotos.

- **Os álbuns recentes inteiros, com a primeira lâmina.** Na tela inicial, cada álbum aparece num
  cartão completo, com a primeira lâmina, o nome, quando foi aberto e quantas lâminas e fotos tem;
  um álbum ainda sem lâminas aparece como um álbum em branco com o nome. Experimente: abra um
  álbum, feche com Ctrl+W e veja o cartão dele na tela inicial.
- **As lâminas lado a lado.** As lâminas vizinhas aparecem dos lados da que você está editando, e a
  roda do mouse ou dois dedos no touchpad passeiam pelo álbum de lado. Um clique numa vizinha a
  traz para o meio; passando o mouse, aparecem os atalhos dela e a alça para mudar a lâmina de
  lugar. Experimente: role a roda do mouse sobre a lâmina e clique na vizinha da direita.
  <!-- imagem: rolo.png -->
- **Fotos de uma lâmina para outra.** Arraste a foto de um quadro para outra lâmina: num quadro
  vazio ela entra, num quadro com foto as duas trocam de lugar, e fora dos quadros ela entra como
  uma foto a mais e a lâmina se rearruma. O quadro de onde ela saiu fica vazio, e um Ctrl+Z desfaz
  nas duas lâminas. Experimente: arraste uma foto da lâmina para um quadro da vizinha ao lado.
  <!-- imagem: fotos-entre-laminas.png -->
- **A linha do tempo acompanha você.** As fotos da lâmina em que você está ficam destacadas e no
  meio da linha do tempo, e um clique numa foto leva à lâmina dela. Experimente: passe para a
  próxima lâmina com Page Down e clique numa foto da linha do tempo.
  <!-- imagem: linha-do-tempo.png -->
- **Faixa de layouts.** Aperte L (ou o botão Layouts sobre a lâmina) e os layouts descem numa
  faixa por cima da lâmina; pare o mouse num deles e a lâmina já mostra as fotos nos lugares novos,
  sem gravar nada. Clique para ficar com ele; Esc ou um clique fora fecha sem mudar nada.
  Experimente: aperte L e passe o mouse pelos layouts.
  <!-- imagem: faixa-de-layouts.png; comando: ver.faixa-layouts -->
- **Painel de fotos em grade.** O painel mostra várias fileiras e rola para baixo; o zoom do
  cabeçalho (ou Ctrl+roda) deixa as miniaturas menores ou maiores, a divisória dá mais altura ao
  painel e um retângulo arrastado no vão entre as fotos seleciona várias de uma vez. Experimente:
  arraste um retângulo entre as fotos e veja quantas foram selecionadas.
  <!-- imagem: painel-de-fotos.png -->
- **O álbum começa em branco.** O Novo álbum abre com todas as fotos no painel e nenhuma lâmina:
  você monta aos poucos, soltando as fotos na lâmina vazia. Quer tudo de uma vez? "Montar o
  álbum…", no cabeçalho do painel de fotos, monta as fotos que sobraram, e um Ctrl+Z desfaz tudo.
  Experimente: crie um álbum com uma pasta de fotos e solte três fotos na lâmina vazia.
  <!-- imagem: album-em-branco.png; comando: lamina.montar-album -->
- **Muitas fotos sem travar.** Importar 1.000 fotos de uma vez não fecha mais o Lamina: ele lê
  poucas fotos por vez, usa bem menos memória e as miniaturas vão aparecendo sem travar a janela.
  Experimente: Arquivo → "Importar fotos…" e escolha uma pasta grande.
  <!-- imagem: muitas-fotos.png; comando: arquivo.importar -->
- **Janela enxuta.** A barra de cima ficou só com o essencial, e o painel da direita começa
  escondido, deixando a lâmina maior; ele aparece sozinho quando você ajusta uma foto ou escreve um
  texto. O quadro vazio ganhou um X na tela, que não sai na exportação. Experimente: clique em
  Painel para mostrar ou esconder; o Lamina lembra a escolha.
  <!-- imagem: janela-enxuta.png; comando: ver.painel -->
- **Atalhos do teclado.** Ajuda → "Atalhos do teclado…" mostra todos os atalhos e os gestos do
  mouse, com busca. Experimente: aperte F1 e procure “trocar”.
  <!-- imagem: atalhos.png; comando: ajuda.atalhos -->
- **Lupa e tamanho real no modo organizar.** A lupa (Z) aproxima no ponto clicado, até 8×; o 1:1
  mostra a lâmina no tamanho de verdade, calibrado com um cartão de banco, e Ajustar (0) volta à
  lâmina inteira. Cada botão da paleta diz o que faz e a tecla. Experimente: aperte O para
  organizar a lâmina, depois Z, e clique num rosto.
  <!-- imagem: zoom.png; comando: lamina.organizar -->
- **Opacidade, borda e cantos da foto.** Com uma foto clicada, a seção Foto ganhou opacidade,
  borda, cor da borda e cantos, que mudam a lâmina na hora; a foto translúcida deixa ver o fundo, e
  o JPEG e o PDF saem iguais à tela. Experimente: clique numa foto sobre um fundo colorido e leve a
  opacidade para 50%.
  <!-- imagem: opacidade.png -->
- **Clique duplo volta ao neutro.** Nos controles de endireitar, brilho e contraste da foto, dois
  cliques no seletor levam o valor de volta ao zero, e um Ctrl+Z desfaz só isso. Experimente: em
  "Ajustar a foto…", arraste o brilho e dê dois cliques no seletor.
  <!-- imagem: seletor-neutro.png; comando: foto.ajustar -->
- **No Mac, o Lamina na pasta Aplicativos.** Instalado, o Lamina fica na pasta Aplicativos, como os
  outros programas, e dois cliques num arquivo .lamina abrem o álbum, com o Lamina aberto ou
  fechado. Experimente: no Finder, dê dois cliques num álbum .lamina.
  <!-- imagem: mac.png -->
- **O que há de novo.** Depois de atualizar, o Lamina mostra o que mudou desde a sua versão, com o
  que experimentar em cada novidade, numa animação curta. Experimente: Ajuda → "O que há de novo…".
  <!-- imagem: novidades.png; comando: ajuda.novidades -->

## [0.1.0-beta.4] - 2026-10-20

Esta é a versão da primeira rodada de testes com fotógrafos.

- **Tela inicial com os seus álbuns.** Os últimos álbuns aparecem em cartões com miniatura, e um
  clique abre o álbum na lâmina em que você parou. Dá para fixar os favoritos no topo; Ctrl+W ou o
  botão Início voltam para essa tela.
- **Novo álbum em três passos.** Escolha o tamanho (formatos desenhados, com os tamanhos de
  mercado, ou medidas próprias em mm, cm ou polegadas), depois como as fotos entram (cortar nada,
  pouco ou à vontade, com a cor do fundo e da borda num seletor) e por fim as fotos e onde salvar.
  O álbum já é gravado na pasta e dá para começar sem fotos.
- **Movimento.** Trocar o layout, espelhar, trocar fotos e desfazer mostram cada foto deslizando
  para o lugar novo, e ir para outra lâmina desliza na direção certa. Prefere sem animação?
  Visualizar → "Reduzir movimento".
- **Mais rápido.** Cada mudança aparece na tela em menos de 40 milésimos de segundo, e as fotos
  ficam nítidas sem travar a navegação.
- **Arrastar com retorno.** Arrastar uma foto mostra a parte dela que está no quadro, acende o
  quadro de destino e a foto "pousa" ao soltar; reordenar lâminas abre espaço entre as vizinhas.
- **Ajustes da foto.** Endireite uma foto torta com precisão de 0,1° (até 45°), com a grade de
  referência, e ajuste brilho, contraste e preto e branco. A foto nunca mostra canto vazio ao
  girar e sai igual na exportação. Clique direito na foto → "Ajustar a foto…"; "Restaurar a foto"
  volta ao original.
- **Painel Detalhes.** A antiga aba Info mostra o álbum quando nada está selecionado e, com uma
  foto, os dados da câmera (velocidade, abertura, ISO e lente). Alt+4 esconde o painel e dá mais
  espaço para a lâmina.
- **Organizar a lâmina à mão.** Dois cliques fora dos quadros (ou a tecla O) abrem a lâmina em
  destaque: selecione vários quadros, mova, amplie até sangrar, alinhe, iguale e distribua, com
  réguas, zoom e o aviso "= 9 cm" quando as medidas batem. Esc conclui.
- **Importar fotos num álbum aberto.** Arquivo → "Importar fotos…" (Ctrl+I), o botão "+ Fotos" ou
  arrastar fotos e pastas do Explorador para a janela. Fotos repetidas não entram duas vezes, e
  Ctrl+Z desfaz.
- **Soltar fotos numa lâmina.** Arraste fotos da faixa para o "+ Lâmina nova", entre duas lâminas
  ou para uma lâmina pronta, e o Lamina monta o layout com elas.
- **Programa menor.** O instalador ficou mais leve, e as exportações saem iguais.
- **Atenção:** álbuns salvos nesta versão não abrem nas betas anteriores. Os álbuns antigos abrem
  normalmente aqui.
- **Experimente sem medo.** Na tela de boas-vindas, "Experimentar com o álbum de exemplo" abre um
  álbum pronto, com fotos livres de uma viagem. Mexa à vontade: ele é uma cópia só sua.
- **Mande sugestões e problemas sem criar conta.** Ajuda → "Enviar sugestão ou problema…" abre um
  formulário curto no navegador, já com a versão do Lamina. Se algo der errado, Ajuda → "Abrir
  pasta de registros" mostra o arquivo para anexar. O Lamina não envia nada sozinho.
- **Erros que dá para entender.** Quando algo dá errado, a mensagem diz o que aconteceu e o que
  fazer agora. O texto técnico fica em "Mostrar detalhes".
- **Nada de tela travada.** Se uma tarefa em segundo plano falhar, o Lamina volta a responder e
  avisa, em vez de ficar parado em "Exportando…".

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
