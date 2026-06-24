#!/usr/bin/env sh
# Installs the saya binary from GitHub Releases.
#
#   curl -fsSL https://raw.githubusercontent.com/Stasshe/saya/main/install.sh | sh
#
# Env overrides:
#   SAYA_VERSION      release tag to install, e.g. "v0.1.0" (default: latest)
#   SAYA_INSTALL_DIR  install directory (default: /usr/local/bin)
set -eu

REPO="Stasshe/saya"
INSTALL_DIR="${SAYA_INSTALL_DIR:-/usr/local/bin}"
VERSION="${SAYA_VERSION:-latest}"

os="$(uname -s)"
if [ "$os" != "Linux" ]; then
  echo "saya only supports Linux (got: $os)" >&2
  exit 1
fi

arch="$(uname -m)"
case "$arch" in
  x86_64) target="x86_64-unknown-linux-musl" ;;
  aarch64 | arm64) target="aarch64-unknown-linux-musl" ;;
  *)
    echo "unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

if [ "$VERSION" = "latest" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
    grep '"tag_name"' | head -1 | cut -d'"' -f4)"
  if [ -z "$VERSION" ]; then
    echo "could not resolve latest release tag" >&2
    exit 1
  fi
fi

archive="saya-${VERSION}-${target}.tar.gz"
url="https://github.com/${REPO}/releases/download/${VERSION}/${archive}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "downloading ${url}"
curl -fsSL "$url" -o "${tmp_dir}/${archive}"
tar -xzf "${tmp_dir}/${archive}" -C "$tmp_dir"

bin_path="${tmp_dir}/saya"
if [ ! -f "$bin_path" ]; then
  bin_path="$(find "$tmp_dir" -type f -name saya | head -1)"
fi
if [ -z "${bin_path:-}" ] || [ ! -f "$bin_path" ]; then
  echo "saya binary not found in downloaded archive" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
if [ -w "$INSTALL_DIR" ]; then
  install -m 755 "$bin_path" "${INSTALL_DIR}/saya"
else
  sudo install -m 755 "$bin_path" "${INSTALL_DIR}/saya"
fi

echo "saya ${VERSION} installed to ${INSTALL_DIR}/saya"
echo "next: saya install <package>"
