# Instalar o Lamina

O Lamina é um programa gratuito para diagramar álbuns de fotos. Este guia mostra como instalar no
Windows, no Mac e no Linux, em poucos minutos.

**Antes de começar:** na página da versão, role até **Assets** (arquivos) e baixe o arquivo do seu
sistema. Os nomes têm `beta` quando é uma versão de teste.

| Sistema | Arquivo para baixar |
|---|---|
| Windows 10 ou 11 | o que termina em `Setup.exe` (por exemplo, `Lamina-win-beta-Setup.exe`) |
| Mac (Intel ou Apple) | o que termina em `Setup.pkg` (por exemplo, `Lamina-osx-beta-Setup.pkg`) |
| Linux | o que termina em `.AppImage` (por exemplo, `Lamina-linux-beta.AppImage`) |

## Por que o sistema avisa na primeira vez

O Lamina ainda não tem a assinatura paga da Microsoft e da Apple. Por isso, na primeira vez, o
Windows e o Mac dizem que não reconhecem o programa. **O aviso não quer dizer vírus:** o código do
Lamina é aberto e qualquer pessoa pode conferir. Depois da instalação, as atualizações chegam
pelo próprio Lamina, sem esse aviso.

## Windows

1. Abra o arquivo `Setup.exe` que você baixou (clique duas vezes nele, na pasta Downloads).
2. Se aparecer uma janela azul **"O Windows protegeu o computador"**:
   1. clique em **Mais informações** (o texto pequeno, embaixo da mensagem);
   2. clique no botão **Executar assim mesmo**.
3. A instalação leva poucos segundos e não pede senha de administrador. No fim, o Lamina abre
   sozinho.
4. Nas próximas vezes, abra pelo atalho **Lamina** na área de trabalho ou no menu Iniciar.

## Mac

1. Abra o arquivo `Setup.pkg` que você baixou (na pasta Transferências) e siga o instalador.
2. Se o Mac disser que **não pode verificar o desenvolvedor** ou que o Lamina **não foi aberto**:
   1. clique em **OK** (ou **Concluído**);
   2. abra **Ajustes do Sistema** (o ícone de engrenagem) → **Privacidade e Segurança**;
   3. role até o fim e, ao lado da mensagem sobre o Lamina, clique em **Abrir mesmo assim**;
   4. confirme com a sua senha do Mac, se ele pedir.
3. Isso só acontece na primeira vez. Depois, abra o Lamina pela pasta **Aplicativos** ou pelo
   Launchpad.

## Linux

1. Baixe o arquivo `.AppImage` e guarde numa pasta sua (por exemplo, `Aplicativos` na sua pasta
   pessoal).
2. Clique com o botão direito no arquivo → **Propriedades** → **Permissões** e marque **Permitir
   executar o arquivo como programa**.
3. Clique duas vezes no arquivo para abrir. Na primeira vez, o Lamina cria o atalho no menu de
   aplicativos.
4. Se não abrir, a sua distribuição pode precisar do pacote `libfuse2` (no Ubuntu: `sudo apt
   install libfuse2`).

## Primeiro uso

Na tela de boas-vindas, clique em **Experimentar com o álbum de exemplo**. Abre um álbum pronto,
com fotos livres, para você ver como o Lamina funciona: troque layouts com as setas ← e →, ponha
um título, exporte. Nada que você fizer ali afeta as suas fotos.

Quando quiser, volte e escolha **Novo álbum com minhas fotos…**: o Lamina monta o álbum a partir
de uma pasta de fotos.

## Atualizações

O Lamina procura versões novas sozinho, baixa em segundo plano e avisa com **Reiniciar e
atualizar**. Para receber as versões de teste (beta), use **Ajuda → Receber versões de teste
(beta)**.

## Encontrou um problema ou tem uma ideia?

No Lamina, use **Ajuda → Enviar sugestão ou problema…**. Abre um formulário curto no navegador,
sem precisar de conta. Se algo deu errado, anexe o registro: **Ajuda → Abrir pasta de registros**
mostra o arquivo `lamina.log`. Não anexe fotos de clientes.

## Para quem prefere o terminal

Estes comandos instalam o Lamina sem o aviso do sistema (o antivírus continua examinando os
arquivos):

- **Windows (PowerShell):**
  `irm https://raw.githubusercontent.com/alberthpalhares/lamina-albuns/main/crates/lamina-app/dist/instalar.ps1 | iex`
  (para a versão de teste, rode antes `$env:LAMINA_BETA = 1`);
- **Mac e Linux:**
  `curl -fsSL https://raw.githubusercontent.com/alberthpalhares/lamina-albuns/main/crates/lamina-app/dist/instalar.sh | sh -s -- --beta`
  (a versão de teste; sem o `-s -- --beta`, a estável, quando houver).
