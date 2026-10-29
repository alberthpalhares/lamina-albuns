// Página do Lamina (spec VP): escolhe o sistema de quem visita, aponta os botões para o
// instalador da versão mais nova (a estável; sem estável, a de teste), troca as abas de
// instalação e copia os comandos.
const REPO = 'alberthpalhares/lamina-albuns';
const VERSOES = `https://github.com/${REPO}/releases`;

const SISTEMAS = {
  windows: {nome: 'Windows', arquivo: /^Lamina-win(-beta)?-Setup\.exe$/},
  mac: {nome: 'Mac', arquivo: /^Lamina-osx(-beta)?-Setup\.pkg$/},
  linux: {nome: 'Linux', arquivo: /^Lamina-linux(-beta)?\.AppImage$/},
};

function sistemaDeQuemVisita() {
  const plataforma = (navigator.userAgentData?.platform || navigator.platform || '').toLowerCase();
  const agente = navigator.userAgent.toLowerCase();
  if (/android|iphone|ipad/.test(agente)) return null;
  if (plataforma.includes('win') || agente.includes('windows')) return 'windows';
  if (plataforma.includes('mac') || agente.includes('mac os')) return 'mac';
  if (plataforma.includes('linux') || agente.includes('linux')) return 'linux';
  return null;
}

async function versaoMaisNova() {
  const resposta = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=10`, {
    headers: {Accept: 'application/vnd.github+json'},
  });
  if (!resposta.ok) throw new Error(`GitHub respondeu ${resposta.status}`);
  const versoes = (await resposta.json()).filter((v) => !v.draft);
  return versoes.find((v) => !v.prerelease) || versoes[0];
}

function abrirAba(sistema, focar = false) {
  for (const id of Object.keys(SISTEMAS)) {
    const aba = document.getElementById(`aba-${id}`);
    const ativa = id === sistema;
    aba.setAttribute('aria-selected', String(ativa));
    aba.tabIndex = ativa ? 0 : -1;
    document.getElementById(`painel-${id}`).hidden = !ativa;
    if (ativa && focar) aba.focus();
  }
}

function prepararAbas() {
  const ids = Object.keys(SISTEMAS);
  for (const id of ids) {
    const aba = document.getElementById(`aba-${id}`);
    aba.addEventListener('click', () => abrirAba(id));
    aba.addEventListener('keydown', (e) => {
      const passo = {ArrowRight: 1, ArrowLeft: -1}[e.key];
      if (!passo) return;
      e.preventDefault();
      abrirAba(ids[(ids.indexOf(id) + passo + ids.length) % ids.length], true);
    });
  }
}

function prepararCopiar() {
  for (const botao of document.querySelectorAll('.copiar')) {
    botao.addEventListener('click', async () => {
      const texto = botao.previousElementSibling.textContent;
      try {
        await navigator.clipboard.writeText(texto);
        botao.textContent = 'Copiado';
        botao.dataset.ok = '';
      } catch {
        botao.textContent = 'Selecione e copie';
      }
      setTimeout(() => { botao.textContent = 'Copiar'; delete botao.dataset.ok; }, 2200);
    });
  }
}

async function prepararDownloads(sistema) {
  const botao = document.getElementById('baixar');
  const rotulo = document.getElementById('baixar-rotulo');
  const detalhe = document.getElementById('baixar-detalhe');
  if (sistema) rotulo.textContent = `Baixar para ${SISTEMAS[sistema].nome}`;
  let versao;
  try {
    versao = await versaoMaisNova();
  } catch {
    botao.href = VERSOES;
    return;
  }
  const link = (id) => versao.assets.find((a) => SISTEMAS[id].arquivo.test(a.name))?.browser_download_url;
  for (const a of document.querySelectorAll('.link-baixar')) {
    a.href = link(a.dataset.sistema) || versao.html_url;
  }
  const numero = versao.tag_name.replace(/^v/, '');
  detalhe.textContent = `versão ${numero}${versao.prerelease ? ' (de teste)' : ''} · grátis · sem conta`;
  if (sistema && link(sistema)) {
    botao.href = link(sistema);
  } else {
    botao.href = '#instalar';
  }
}

document.addEventListener('DOMContentLoaded', () => {
  const sistema = sistemaDeQuemVisita();
  prepararAbas();
  prepararCopiar();
  if (sistema) abrirAba(sistema);
  prepararDownloads(sistema);
});
