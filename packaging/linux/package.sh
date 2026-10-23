#!/usr/bin/env bash
# Build Linux release packages (deb, rpm, pacman, AppImage, tar.xz) from a release binary.
# Usage: package.sh <binary> <version> <x86_64|aarch64> <outdir>
set -euo pipefail

BIN="${1:?usage: package.sh <binary> <version> <arch> <outdir>}"
VERSION="${2:?}"
ARCH="${3:?}"
OUTDIR="${4:?}"

case "$ARCH" in
  x86_64) DEB_ARCH=amd64 ;;
  aarch64) DEB_ARCH=arm64 ;;
  *)
    echo "unsupported arch: $ARCH" >&2
    exit 1
    ;;
esac

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUTDIR"

DEB_VERSION="${VERSION//-/\~}"
RPM_VERSION="${VERSION//-/\~}"
PACMAN_VERSION="${VERSION//-/_}"

DESKTOP="$ROOT/packaging/linux/screenscan.desktop"
ICON_SVG="$ROOT/packaging/linux/screenscan.svg"

TREE="$WORK/tree"
install -Dm755 "$BIN" "$TREE/usr/bin/screenscan"
install -Dm644 "$DESKTOP" "$TREE/usr/share/applications/screenscan.desktop"
install -Dm644 "$ICON_SVG" "$TREE/usr/share/icons/hicolor/scalable/apps/screenscan.svg"

# ── deb / rpm ────────────────────────────────────────────────────────────────
cat > "$WORK/nfpm-deb.yaml" <<EOF
name: screenscan
arch: ${DEB_ARCH}
platform: linux
version: "${DEB_VERSION}"
section: graphics
priority: optional
maintainer: Spinach5
vendor: Spinach5
homepage: https://github.com/Spinach5/ScreenScan
description: |
  Cross-platform screenshot, annotation and pin tool.
depends:
  - libc6
  - libgcc-s1
  - libgl1
  - libegl1
  - libgbm1
  - libdrm2
  - libpipewire-0.3-0
  - libwayland-client0
  - libwayland-cursor0
  - libwayland-egl1
  - libxkbcommon0
  - libx11-6
  - libxcb1
  - libxcb-randr0
  - libxcb-shm0
  - libxcb-shape0
  - libxcb-xfixes0
  - libxcb-sync1
contents:
  - src: ${TREE}/usr/bin/screenscan
    dst: /usr/bin/screenscan
    file_info:
      mode: 0755
  - src: ${DESKTOP}
    dst: /usr/share/applications/screenscan.desktop
    file_info:
      mode: 0644
  - src: ${ICON_SVG}
    dst: /usr/share/icons/hicolor/scalable/apps/screenscan.svg
    file_info:
      mode: 0644
deb:
  compression: xz
EOF
nfpm package --config "$WORK/nfpm-deb.yaml" --packager deb \
  --target "$OUTDIR/screenscan_${DEB_VERSION}_${DEB_ARCH}.deb"

cat > "$WORK/nfpm-rpm.yaml" <<EOF
name: screenscan
arch: ${ARCH}
platform: linux
version: "${RPM_VERSION}"
section: graphics
priority: optional
maintainer: Spinach5
vendor: Spinach5
homepage: https://github.com/Spinach5/ScreenScan
description: |
  Cross-platform screenshot, annotation and pin tool.
depends:
  - libc.so.6()(64bit)
  - libgcc_s.so.1()(64bit)
  - libGL.so.1()(64bit)
  - libEGL.so.1()(64bit)
  - libgbm.so.1()(64bit)
  - libdrm.so.2()(64bit)
  - libpipewire-0.3.so.0()(64bit)
  - libwayland-client.so.0()(64bit)
  - libwayland-cursor.so.0()(64bit)
  - libwayland-egl.so.1()(64bit)
  - libxkbcommon.so.0()(64bit)
  - libX11.so.6()(64bit)
  - libxcb.so.1()(64bit)
contents:
  - src: ${TREE}/usr/bin/screenscan
    dst: /usr/bin/screenscan
    file_info:
      mode: 0755
  - src: ${DESKTOP}
    dst: /usr/share/applications/screenscan.desktop
    file_info:
      mode: 0644
  - src: ${ICON_SVG}
    dst: /usr/share/icons/hicolor/scalable/apps/screenscan.svg
    file_info:
      mode: 0644
rpm:
  group: Applications/Graphics
  compression: xz
EOF
nfpm package --config "$WORK/nfpm-rpm.yaml" --packager rpm \
  --target "$OUTDIR/screenscan-${RPM_VERSION}-1.${ARCH}.rpm"

# ── pacman (built manually: no Arch toolchain required) ──────────────────────
cat > "$TREE/.PKGINFO" <<EOF
pkgname = screenscan
pkgbase = screenscan
pkgver = ${PACMAN_VERSION}-1
pkgdesc = Cross-platform screenshot, annotation and pin tool
url = https://github.com/Spinach5/ScreenScan
builddate = $(date +%s)
packager = Spinach5
size = $(du -sb "$TREE/usr" | cut -f1)
arch = ${ARCH}
depend = pipewire
depend = mesa
depend = libglvnd
depend = libxkbcommon
depend = wayland
depend = libx11
depend = libxcb
depend = libdrm
EOF
(
  cd "$TREE"
  bsdtar -czf .MTREE --format=mtree \
    --options='!all,use-set,type,uid,gid,mode,time,size,md5,sha256,link' \
    .PKGINFO usr
  bsdtar --uid 0 --gid 0 -cf - .PKGINFO .MTREE usr \
    | zstd -q -T0 -19 -o "$OUTDIR/screenscan-${PACMAN_VERSION}-1-${ARCH}.pkg.tar.zst"
)

# ── AppImage ─────────────────────────────────────────────────────────────────
APPDIR="$WORK/AppDir"
install -Dm755 "$BIN" "$APPDIR/usr/bin/screenscan"
install -Dm644 "$DESKTOP" "$APPDIR/screenscan.desktop"
install -Dm644 "$DESKTOP" "$APPDIR/usr/share/applications/screenscan.desktop"
rsvg-convert -w 256 -h 256 "$ICON_SVG" -o "$APPDIR/screenscan.png"
install -Dm644 "$APPDIR/screenscan.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/screenscan.png"
ln -sf screenscan.png "$APPDIR/.DirIcon"
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/screenscan" "$@"
EOF
chmod +x "$APPDIR/AppRun"

TOOL="$WORK/appimagetool.AppImage"
curl -fsSL -o "$TOOL" \
  "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-${ARCH}.AppImage"
chmod +x "$TOOL"
ARCH="$ARCH" "$TOOL" --appimage-extract-and-run --no-appstream \
  "$APPDIR" "$OUTDIR/ScreenScan-${VERSION}-linux-${ARCH}.AppImage"

# ── tar.xz ───────────────────────────────────────────────────────────────────
STAGE="$WORK/tar/screenscan"
install -Dm755 "$BIN" "$STAGE/screenscan"
install -Dm644 "$DESKTOP" "$STAGE/screenscan.desktop"
install -Dm644 "$ICON_SVG" "$STAGE/screenscan.svg"
tar -C "$WORK/tar" -cJf "$OUTDIR/ScreenScan-${VERSION}-linux-${ARCH}.tar.xz" screenscan

ls -l "$OUTDIR"
