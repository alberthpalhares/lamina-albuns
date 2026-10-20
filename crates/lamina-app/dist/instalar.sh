#!/bin/sh
# Instala o Lamina no Mac ou no Linux pelo terminal (spec DS, regra 14), sem senha e sem
# administrador, só na pasta do usuário:
#
#   curl -fsSL https://raw.githubusercontent.com/alberthpalhares/lamina-albuns/main/crates/lamina-app/dist/instalar.sh | sh
#   curl -fsSL .../instalar.sh | sh -s -- --beta      (a versão de teste mais nova)
#
# Sem versão estável publicada, instala a versão de teste mais nova e avisa (Rodada 0, R0-15:
# o endereço "latest" do GitHub não vê versões de teste e o comando falhava).
#
# O que o terminal baixa não ganha a marca de "baixado da internet", então o Mac não mostra o
# aviso de app não verificado. O Lamina não fica verificado pela Apple: o aviso só não aparece.
# A soma SHA-256 de cada pacote é conferida antes de instalar.
#
# Para os testes: LAMINA_FONTE troca de onde vêm os arquivos (uma pasta ou um endereço).

set -eu

REPO="alberthpalhares/lamina-albuns"
BETA=0
for arg in "$@"; do
    case "$arg" in
        --beta) BETA=1 ;;
    esac
done

falha() {
    printf '%s\n' "$1" >&2
    exit 1
}

case "$(uname -s)" in
    Darwin) SISTEMA=mac; PADRAO='osx.*Portable\.zip$' ;;
    Linux)
        [ "$(uname -m)" = "x86_64" ] || falha "Este sistema não é suportado pelo Lamina (no Linux, precisa ser de 64 bits x86)."
        SISTEMA=linux; PADRAO='\.AppImage$' ;;
    *) falha "Este sistema não é suportado pelo Lamina (use o Windows, o Mac ou o Linux)." ;;
esac

if [ -n "${LAMINA_FONTE:-}" ]; then
    FONTE="$LAMINA_FONTE"
else
    # A versão do primeiro "tag_name" da resposta da API do GitHub (vazio se não houver).
    # Com GITHUB_TOKEN (só o CI tem), a consulta não esbarra no limite de 60 por hora que o
    # GitHub dá a cada endereço sem login; as máquinas do CI dividem o endereço.
    tag_de() {
        if [ -n "${GITHUB_TOKEN:-}" ]; then
            curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" "https://api.github.com/repos/$REPO/$1" 2>/dev/null
        else
            curl -fsSL "https://api.github.com/repos/$REPO/$1" 2>/dev/null
        fi | grep -m1 '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/' || true
    }
    TAG=""
    [ "$BETA" = 1 ] || TAG=$(tag_de releases/latest)
    if [ -z "$TAG" ]; then
        TAG=$(tag_de "releases?per_page=20")
        [ "$BETA" = 1 ] || [ -z "$TAG" ] \
            || printf 'Ainda não há versão estável do Lamina: instalando a versão de teste %s.\n' "$TAG"
    fi
    [ -n "$TAG" ] || falha "Não foi possível baixar o Lamina. Confira a internet e tente de novo."
    FONTE="https://github.com/$REPO/releases/download/$TAG"
fi

TEMP=$(mktemp -d)
trap 'rm -rf "$TEMP"' EXIT

baixa() {
    if [ -d "$FONTE" ]; then
        cp "$FONTE/$1" "$TEMP/$1" 2>/dev/null
    else
        curl -fsSL -o "$TEMP/$1" "$FONTE/$1"
    fi || falha "Não foi possível baixar o Lamina. Confira a internet e tente de novo."
}

printf 'Baixando o Lamina…\n'
baixa SHA256SUMS
LINHA=$(grep -E "$PADRAO" "$TEMP/SHA256SUMS" | head -n 1) || true
[ -n "$LINHA" ] || falha "Não foi possível baixar o Lamina: a versão publicada não tem o pacote deste sistema."
SOMA=$(printf '%s' "$LINHA" | awk '{print $1}')
ARQUIVO=$(printf '%s' "$LINHA" | awk '{print $2}' | sed 's/^\*//')
baixa "$ARQUIVO"

if command -v sha256sum >/dev/null 2>&1; then
    CONFERIDA=$(sha256sum "$TEMP/$ARQUIVO" | awk '{print $1}')
else
    CONFERIDA=$(shasum -a 256 "$TEMP/$ARQUIVO" | awk '{print $1}')
fi
if [ "$CONFERIDA" != "$SOMA" ]; then
    rm -f "$TEMP/$ARQUIVO"
    falha "O arquivo baixado não confere; nada foi instalado."
fi

if [ "$SISTEMA" = mac ]; then
    DESTINO="$HOME/Applications"
    mkdir -p "$DESTINO" "$TEMP/app"
    ditto -x -k "$TEMP/$ARQUIVO" "$TEMP/app" || falha "Não foi possível abrir o pacote do Lamina; nada foi instalado."
    APP=$(find "$TEMP/app" -maxdepth 1 -name '*.app' | head -n 1)
    [ -n "$APP" ] || falha "Não foi possível abrir o pacote do Lamina; nada foi instalado."
    rm -rf "$DESTINO/Lamina.app"
    mv "$APP" "$DESTINO/Lamina.app"
    # O programa precisa poder rodar mesmo se o pacote perdeu a permissão no caminho.
    chmod +x "$DESTINO/Lamina.app/Contents/MacOS/"* 2>/dev/null || true
    printf 'Pronto! O Lamina está em %s/Lamina.app. Abra pelo Launchpad ou pelo Finder.\n' "$DESTINO"
else
    DESTINO="$HOME/.local/bin"
    mkdir -p "$DESTINO"
    chmod +x "$TEMP/$ARQUIVO"
    mv -f "$TEMP/$ARQUIVO" "$DESTINO/Lamina.AppImage"
    printf 'Pronto! O Lamina está em %s/Lamina.AppImage.\n' "$DESTINO"
    printf 'Abra uma vez por ele: o atalho entra no menu de aplicativos.\n'
fi
