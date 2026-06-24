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
checksum_url="${url}.sha256"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "downloading ${url}"
curl -fsSL "$url" -o "${tmp_dir}/${archive}"

if ! command -v sha256sum >/dev/null 2>&1; then
  echo "sha256sum is required to verify the downloaded archive" >&2
  exit 1
fi

echo "downloading ${checksum_url}"
curl -fsSL "$checksum_url" -o "${tmp_dir}/${archive}.sha256"
(cd "$tmp_dir" && sha256sum -c "${archive}.sha256")

tar -tzf "${tmp_dir}/${archive}" > "${tmp_dir}/archive.list"
unsafe_archive=0
while IFS= read -r entry; do
  case "$entry" in
    "" | /* | ../* | */../* | .. | */..)
      unsafe_archive=1
      ;;
  esac
done < "${tmp_dir}/archive.list"
if [ "$unsafe_archive" -ne 0 ]; then
  echo "downloaded archive contains unsafe paths" >&2
  exit 1
fi

tar -xzf "${tmp_dir}/${archive}" -C "$tmp_dir"

bin_path="${tmp_dir}/saya"
if [ -z "${bin_path:-}" ] || [ ! -f "$bin_path" ] || [ -L "$bin_path" ]; then
  echo "downloaded archive must contain a regular ./saya binary" >&2
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
