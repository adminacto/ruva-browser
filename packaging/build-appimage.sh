#!/bin/bash
# Build AppImage for Ruva Browser
set -e

APP_NAME="ruva-browser"
APPDIR="/tmp/${APP_NAME}-AppDir"
VERSION=$(grep 'version' native/Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

echo "=== Building AppImage v${VERSION} ==="

# 1. Release build
echo "[1/5] Building release..."
cd native
cargo build --release
strip target/release/ruva-browser
cd ..

# 2. Create AppDir structure
echo "[2/5] Creating AppDir..."
rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/256x256/apps"

# 3. Copy files
echo "[3/5] Copying files..."
cp native/target/release/ruva-browser "${APPDIR}/usr/bin/"
cp native/ui/ntp.html "${APPDIR}/usr/share/ruva-browser/ntp.html" 2>/dev/null || true
cp native/ui/settings.html "${APPDIR}/usr/share/ruva-browser/settings.html" 2>/dev/null || true
cp native/ui/setup.html "${APPDIR}/usr/share/ruva-browser/setup.html" 2>/dev/null || true

# 4. Desktop file
cat > "${APPDIR}/usr/share/applications/${APP_NAME}.desktop" << EOF
[Desktop Entry]
Name=Ruva Browser
Comment=Web browser with AI assistant
Exec=ruva-browser
Icon=ruva-browser
Type=Application
Categories=Network;WebBrowser;
Terminal=false
EOF

cp "${APPDIR}/usr/share/applications/${APP_NAME}.desktop" "${APPDIR}/${APP_NAME}.desktop"

# 5. Create icon (simple SVG)
cat > "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_NAME}.svg" << 'SVGEOF'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
  <defs>
    <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#3b82f6"/>
      <stop offset="100%" style="stop-color:#8b5cf6"/>
    </linearGradient>
  </defs>
  <rect width="256" height="256" rx="48" fill="url(#g)"/>
  <text x="128" y="170" text-anchor="middle" font-family="sans-serif" font-weight="bold" font-size="140" fill="white">R</text>
</svg>
SVGEOF
cp "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_NAME}.svg" "${APPDIR}/${APP_NAME}.svg"

# 6. AppRun
cat > "${APPDIR}/AppRun" << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin/:${PATH}"
exec "${HERE}/usr/bin/ruva-browser" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

echo "[4/5] AppDir ready at ${APPDIR}"
echo ""
echo "To create AppImage, run:"
echo "  wget -O /tmp/appimagetool https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
echo "  chmod +x /tmp/appimagetool"
echo "  ARCH=x86_64 /tmp/appimagetool ${APPDIR} ruva-browser-${VERSION}-x86_64.AppImage"
echo ""
echo "Or install appimagetool: yay -S appimagetool"
