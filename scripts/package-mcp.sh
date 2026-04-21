#!/usr/bin/env bash
set -euo pipefail

# 打包 `semantic-search-mcp` + `resources/`，生成可上传 GitHub Release 的归档文件。
#
# 产物：
# - dist/semantic-search-mcp-<platform>-<arch>.tar.gz   (macOS / Linux)
# - dist/semantic-search-mcp-<platform>-<arch>.zip      (Windows)
#
# 说明：
# - 运行时默认会优先从“可执行文件同级的 resources/”加载模型与 onnxruntime；
#   若你希望自定义资源目录，可设置环境变量 `SEMANTIC_SEARCH_RESOURCES_DIR`。

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

BIN_NAME="semantic-search-mcp"

PLATFORM="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${ARCH}" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
esac

echo "[package] building ${BIN_NAME} (release)"
cargo build --release --bin "${BIN_NAME}"

STAGE_DIR="$(mktemp -d)"
cleanup() { rm -rf "${STAGE_DIR}"; }
trap cleanup EXIT

OUT_DIR="${ROOT_DIR}/dist"
mkdir -p "${OUT_DIR}"

PKG_DIR="${STAGE_DIR}/${BIN_NAME}-${PLATFORM}-${ARCH}"
mkdir -p "${PKG_DIR}"

# 二进制
cp "${ROOT_DIR}/target/release/${BIN_NAME}" "${PKG_DIR}/"

# resources（模型 / tokenizer / onnxruntime）
cp -R "${ROOT_DIR}/resources" "${PKG_DIR}/resources"

# 文档（可选）
if [[ -f "${ROOT_DIR}/README.md" ]]; then
  cp "${ROOT_DIR}/README.md" "${PKG_DIR}/"
fi
if [[ -f "${ROOT_DIR}/README.zh-CN.md" ]]; then
  cp "${ROOT_DIR}/README.zh-CN.md" "${PKG_DIR}/"
fi

ARCHIVE_BASE="${OUT_DIR}/${BIN_NAME}-${PLATFORM}-${ARCH}"

echo "[package] creating archive"
if [[ "${PLATFORM}" == "mingw"* || "${PLATFORM}" == "msys"* || "${PLATFORM}" == "cygwin"* ]]; then
  # Git Bash on Windows（尽量兼容）
  (cd "${STAGE_DIR}" && zip -qr "${ARCHIVE_BASE}.zip" "$(basename "${PKG_DIR}")")
  echo "[package] wrote ${ARCHIVE_BASE}.zip"
else
  (cd "${STAGE_DIR}" && tar --exclude='*.DS_Store' -czf "${ARCHIVE_BASE}.tar.gz" "$(basename "${PKG_DIR}")")
  echo "[package] wrote ${ARCHIVE_BASE}.tar.gz"
fi

