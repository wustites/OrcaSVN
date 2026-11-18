#!/usr/bin/env bash
# Build Linux installers (.deb and .AppImage) for the GPUI client.
#
# Usage:
#   RELEASE_TAG=gpui-v0.8.1-alpha.1 scripts/package-linux-installers.sh
#
# Environment:
#   RELEASE_TAG  version label used in the file names (required)
#   OUT_DIR      output directory (default: dist)
#   BINARY       path to the release binary (default: orcasvn-gpui/target/release/orcasvn-gpui)
#   SKIP_APPIMAGE set to 1 to skip the AppImage (useful on non-x86_64 hosts)
set -euo pipefail

tag="${RELEASE_TAG:?RELEASE_TAG is required}"
out="${OUT_DIR:-dist}"
binary="${BINARY:-orcasvn-gpui/target/release/orcasvn-gpui}"
icons="src-tauri/icons"

if [ ! -x "$binary" ]; then
  echo "binary not found: $binary" >&2
  exit 1
fi

mkdir -p "$out"

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' orcasvn-gpui/Cargo.toml | head -1)
# Debian versions cannot contain '-' before the revision; 0.8.1-alpha.1 -> 0.8.1~alpha.1
deb_version=$(printf '%s' "$version" | sed 's/-/~/g')

root="$out/deb-root"
rm -rf "$root"
mkdir -p "$root/DEBIAN" "$root/usr/bin" "$root/usr/share/applications" \
         "$root/usr/share/icons/hicolor/256x256/apps" "$root/usr/share/doc/orcasvn-gpui"
install -m755 "$binary" "$root/usr/bin/orcasvn-gpui"
install -m644 "$icons/256x256.png" \
  "$root/usr/share/icons/hicolor/256x256/apps/orcasvn-gpui.png"
cp orcasvn-gpui/README.md LICENSE "$root/usr/share/doc/orcasvn-gpui/"

cat > "$root/usr/share/applications/orcasvn-gpui.desktop" <<'EOF'
[Desktop Entry]
Name=OrcaSVN
Comment=SVN client (GPUI, experimental)
Exec=orcasvn-gpui %F
Icon=orcasvn-gpui
Terminal=false
Type=Application
Categories=Development;RevisionControl;
EOF

cat > "$root/DEBIAN/control" <<EOF
Package: orcasvn-gpui
Version: $deb_version
Architecture: amd64
Maintainer: OrcaSVN <noreply@orcasvn.local>
Section: vcs
Priority: optional
Depends: libc6, libgcc-s1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libfontconfig1, libfreetype6
Description: OrcaSVN GPUI client (experimental)
 Native SVN client built with Rust and GPUI Kit.
EOF

chmod -R u+rwX,go+rX "$root"
dpkg-deb --root-owner-group --build "$root" \
  "$out/OrcaSVN-${tag}-linux-x86_64.deb"
rm -rf "$root"

if [ "${SKIP_APPIMAGE:-0}" = "1" ]; then
  echo "skipping AppImage"
  exit 0
fi

appdir="$out/OrcaSVN.AppDir"
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/icons/hicolor/256x256/apps"
install -m755 "$binary" "$appdir/usr/bin/orcasvn-gpui"
install -m644 "$icons/256x256.png" "$appdir/orcasvn-gpui.png"
install -m644 "$icons/256x256.png" \
  "$appdir/usr/share/icons/hicolor/256x256/apps/orcasvn-gpui.png"
cat > "$appdir/orcasvn-gpui.desktop" <<'EOF'
[Desktop Entry]
Name=OrcaSVN
Comment=SVN client (GPUI, experimental)
Exec=orcasvn-gpui
Icon=orcasvn-gpui
Terminal=false
Type=Application
Categories=Development;RevisionControl;
EOF
cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
exec "$APPDIR/usr/bin/orcasvn-gpui" "$@"
EOF
chmod +x "$appdir/AppRun"

tool="$out/.appimagetool"
if [ ! -x "$tool" ]; then
  curl -fsSL -o "$tool" \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$tool"
fi
ARCH=x86_64 "$tool" --appimage-extract-and-run "$appdir" \
  "$out/OrcaSVN-${tag}-linux-x86_64.AppImage"
rm -rf "$appdir"
