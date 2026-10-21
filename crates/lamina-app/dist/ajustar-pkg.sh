#!/bin/sh
# Ajusta o .pkg do Mac que o `vpk pack` gera (spec MA, regra 7; ADR 0009, adendo). Roda no CI do
# Mac, na release e no job "Abrir no Mac":
#
#   sh crates/lamina-app/dist/ajustar-pkg.sh releases/Lamina-osx-Setup.pkg
#
# 1. Só "para todos os usuários": o app vai para a pasta Aplicativos do Mac (/Applications), e
#    não para a pasta Aplicativos da pasta pessoal ("só para mim"), que o Finder não mostra na
#    barra lateral.
# 2. No fim da instalação, o app passa para o usuário que instalou, como um app arrastado para
#    Aplicativos: assim as atualizações automáticas não pedem a senha do Mac. E o Mac registra
#    logo que o Lamina abre os arquivos .lamina.
#
# O pacote não tem assinatura de instalador (o vpk só assina com identidade paga), então nada se
# perde ao refazê-lo.

set -eu

PKG="$1"
TEMP=$(mktemp -d)
trap 'rm -rf "$TEMP"' EXIT

pkgutil --expand "$PKG" "$TEMP/x"

DIST="$TEMP/x/Distribution"
grep -q 'enable_currentUserHome="true"' "$DIST" \
    || { echo "ajustar-pkg: o Distribution do vpk mudou (sem enable_currentUserHome)" >&2; exit 1; }
sed 's/enable_currentUserHome="true"/enable_currentUserHome="false"/' "$DIST" > "$TEMP/dist"
mv "$TEMP/dist" "$DIST"

COMP=$(find "$TEMP/x" -maxdepth 1 -name '*.pkg' | head -n 1)
[ -f "$COMP/Scripts" ] || { echo "ajustar-pkg: o pacote do vpk não tem Scripts" >&2; exit 1; }
mkdir "$TEMP/s"
(cd "$TEMP/s" && gzip -dc "$COMP/Scripts" | cpio -i -d --quiet)
POST="$TEMP/s/postinstall"
[ -f "$POST" ] || { echo "ajustar-pkg: o pacote do vpk não tem postinstall" >&2; exit 1; }

# As linhas novas entram logo depois da primeira (#!/bin/sh), antes de o vpk abrir o app.
{
    head -n 1 "$POST"
    cat <<'NOVO'
# Lamina (spec MA): o app em nome de quem está usando o Mac, e o .lamina registrado.
dono=$(stat -f %Su /dev/console 2>/dev/null || true)
if [ -n "$dono" ] && [ "$dono" != root ] && [ -d "$2/Lamina.app" ]; then
    chown -R "$dono" "$2/Lamina.app" || true
fi
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$2/Lamina.app" || true
NOVO
    tail -n +2 "$POST"
} > "$TEMP/postinstall"
mv "$TEMP/postinstall" "$POST"
chmod 755 "$POST"
(cd "$TEMP/s" && find . | cpio -o -H odc --quiet | gzip -c) > "$COMP/Scripts"

pkgutil --flatten "$TEMP/x" "$TEMP/novo.pkg"
mv "$TEMP/novo.pkg" "$PKG"
echo "ajustar-pkg: $PKG só para todos os usuários, com o app em nome de quem instala"
