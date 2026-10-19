#!/bin/sh
# Instala o Lamina no Mac ou no Linux pelo terminal (spec DS, regra 14), sem senha e sem
# administrador, só na pasta do usuário:
#
#   curl -fsSL https://github.com/alberthpalhares/lamina-codigo/releases/latest/download/instalar.sh | sh
#   curl -fsSL .../instalar.sh | sh -s -- --beta      (a versão de teste mais nova)
#
# O que o terminal baixa não ganha a marca de "baixado da internet", então o Mac não mostra o
# aviso de app não verificado. O Lamina não fica verificado pela Apple: o aviso só não aparece.
# A soma SHA-256 de cada pacote é conferida antes de instalar.
#
# Para os testes: LAMINA_FONTE troca de onde vêm os arquivos (uma pasta ou um endereço).

set -eu

REPO="alberthpalhares/lamina-codigo"
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
elif [ "$BETA" = 1 ]; then
    TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases?per_page=20" 2>/dev/null \
        | grep -m1 '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/') \
        || falha "Não foi possível baixar o Lamina. Confira a internet e tente de novo."
    [ -n "$TAG" ] || falha "Não foi possível baixar o Lamina. Confira a internet e tente de novo."
    FONTE="https://github.com/$REPO/releases/download/$TAG"
else
    FONTE="https://github.com/$REPO/releases/latest/download"
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
    printf 'Pronto! O Lamina está em %s/Lamina.app. Abra pelo Launchpad ou pelo Finder.\n' "$DESTINO"
else
    DESTINO="$HOME/.local/bin"
    mkdir -p "$DESTINO"
    chmod +x "$TEMP/$ARQUIVO"
    mv -f "$TEMP/$ARQUIVO" "$DESTINO/Lamina.AppImage"
    printf 'Pronto! O Lamina está em %s/Lamina.AppImage.\n' "$DESTINO"
    printf 'Abra uma vez por ele: o atalho entra no menu de aplicativos.\n'
fi
