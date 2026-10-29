# Segurança

## Versões cobertas

Recebem correções de segurança a versão estável mais nova e a versão de teste mais nova. O Lamina
se atualiza sozinho, então a correção chega a todos pelo próprio programa.

## Como relatar uma falha

**Não abra uma issue pública.** Use o canal privado do GitHub: aba **Security** deste repositório
→ **Report a vulnerability**. Conte:

- o que acontece e o que poderia acontecer de ruim;
- a versão do Lamina e o sistema (Windows, Mac ou Linux);
- como reproduzir, de preferência com o álbum de exemplo (nunca com fotos de clientes).

A resposta chega em até 7 dias. Quando a correção sair, o relato é publicado com o crédito a quem
encontrou, se a pessoa quiser.

## O que o Lamina faz para ser confiável

- Funciona sem internet e não envia nada sozinho; usa a internet só para buscar atualizações.
- Cada instalador é montado pelo GitHub Actions a partir deste código, e cada versão publica o
  `SHA256SUMS` dos arquivos.
- Os comandos de instalação conferem o SHA-256 antes de instalar.
- O controle por IA só lê e grava nas pastas que você escolher, e cada edição pode ser desfeita.
