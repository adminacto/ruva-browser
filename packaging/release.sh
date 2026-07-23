#!/bin/bash
# Quick release build + package
set -e
cd "$(dirname "$0")/.."

VERSION=$(grep 'version' native/Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
OUTDIR="releases/ruva-browser-${VERSION}"

echo "=== Ruva Browser v${VERSION} Release ==="

# Build
echo "[1/3] Release build..."
cd native
cargo build --release
strip target/release/ruva-browser
cd ..

# Package
echo "[2/3] Creating package..."
rm -rf "${OUTDIR}" "releases/ruva-browser-${VERSION}.tar.gz"
mkdir -p "${OUTDIR}"
cp native/target/release/ruva-browser "${OUTDIR}/"
cp native/ui/*.html "${OUTDIR}/"

cat > "${OUTDIR}/install.sh" << 'INSTALLEOF'
#!/bin/bash
echo "Установка Ruva Browser..."
sudo cp ruva-browser /usr/bin/
sudo mkdir -p /usr/share/ruva-browser
sudo cp *.html /usr/share/ruva-browser/
echo "Готово! Запуск: ruva-browser"
INSTALLEOF
chmod +x "${OUTDIR}/install.sh"

cat > "${OUTDIR}/README.md" << READMEEOF
# Ruva Browser v${VERSION}

## Установка
\`\`\`bash
./install.sh
\`\`\`

## Запуск
\`\`\`bash
ruva-browser
\`\`\`

## Требования
- GTK3
- WebKitGTK 4.1
- curl

## Arch Linux
\`\`\`bash
sudo pacman -S gtk3 webkit2gtk-4.1 curl
\`\`\`
READMEEOF

# Tar.gz
echo "[3/3] Creating tar.gz..."
cd releases
tar czf "ruva-browser-${VERSION}-x86_64.tar.gz" "ruva-browser-${VERSION}/"
cd ..

echo ""
echo "Готово!"
echo "  Бинарник:  ${OUTDIR}/ruva-browser"
echo "  Архив:     releases/ruva-browser-${VERSION}-x86_64.tar.gz"
echo "  Размер:    $(du -sh ${OUTDIR}/ruva-browser | cut -f1) (бинарник)"
echo ""
echo "Отправь releases/ruva-browser-${VERSION}-x86_64.tar.gz"
