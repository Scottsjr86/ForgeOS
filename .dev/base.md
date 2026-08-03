#!/usr/bin/env bash
# Create a source archive of the Git repository containing the current directory.
#
# Preferred interface:
#   python3 .dev/tar
#   python3 .dev/tar --thread ForgeOS_V1
#   python3 .dev/tar --thread ForgeOS_V1 --base 0
#
# State used by .dev/tar:
# - OUT_NAME records the active thread and the last successfully created base.
# - A plain run increments the recorded base number.
# - --thread changes the filename prefix and keeps normal base counting.
# - --base creates that exact non-negative base number.
#
# The archive is written to the repository root. Repository metadata, local
# helper files, prior archives, secrets, caches, dependencies, generated
# runtime data, build outputs, and OS/disk images are omitted.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
cd "$ROOT"

OUT_NAME="${1:-Forge_OS_V1_base_35.tar}"
OUT="$ROOT/$OUT_NAME"

case "$OUT_NAME" in
  /*|*/*)
    echo "Pass only a filename; archives are written to the repository root." >&2
    exit 1
    ;;
esac

if [[ ! "$OUT_NAME" =~ ^[A-Za-z0-9]+([._-][A-Za-z0-9]+)*_base_[0-9]+\.tar$ ]]; then
  echo "Archive name must look like <thread>_base_<n>.tar." >&2
  exit 1
fi

if [ ! -d .git ]; then
  echo "Could not confirm the repository root: $ROOT" >&2
  exit 1
fi

if [ -e "$OUT" ]; then
  echo "Refusing to overwrite existing archive: $OUT" >&2
  exit 1
fi

export COPYFILE_DISABLE=1

tar \
  --exclude='./.git' \
  --exclude='./.git/*' \
  --exclude='./.agents' \
  --exclude='./.agents/*' \
  --exclude='./.codex' \
  --exclude='./.codex/*' \
  --exclude='./.dev' \
  --exclude='./.dev/*' \
  --exclude='*/.idea' \
  --exclude='*/.idea/*' \
  --exclude='*/.vscode' \
  --exclude='*/.vscode/*' \
  --exclude='*.code-workspace' \
  --exclude='.env' \
  --exclude='.env.*' \
  --exclude='*/.env' \
  --exclude='*/.env.*' \
  --exclude='target' \
  --exclude='*/target' \
  --exclude='node_modules' \
  --exclude='*/node_modules' \
  --exclude='.venv' \
  --exclude='*/.venv' \
  --exclude='venv' \
  --exclude='*/venv' \
  --exclude='build' \
  --exclude='*/build' \
  --exclude='dist' \
  --exclude='*/dist' \
  --exclude='out' \
  --exclude='*/out' \
  --exclude='cmake-build-*' \
  --exclude='*/cmake-build-*' \
  --exclude='artifacts' \
  --exclude='*/artifacts' \
  --exclude='exports' \
  --exclude='*/exports' \
  --exclude='logs' \
  --exclude='*/logs' \
  --exclude='tmp' \
  --exclude='*/tmp' \
  --exclude='*.log' \
  --exclude='*/__pycache__' \
  --exclude='*/.pytest_cache' \
  --exclude='*/.ruff_cache' \
  --exclude='*/.mypy_cache' \
  --exclude='*/.cache' \
  --exclude='*/.coverage' \
  --exclude='*/htmlcov' \
  --exclude='*/coverage' \
  --exclude='*.py[cod]' \
  --exclude='*.egg-info' \
  --exclude='*.iso' \
  --exclude='*.img' \
  --exclude='*.qcow2' \
  --exclude='*.raw' \
  --exclude='*.squashfs' \
  --exclude='*.AppImage' \
  --exclude='*.gguf' \
  --exclude='*.safetensors' \
  --exclude='*.onnx' \
  --exclude='*.pt' \
  --exclude='*.pth' \
  --exclude='*.tar' \
  --exclude='*.tar.gz' \
  --exclude='*.tar.*' \
  --exclude='*.tgz' \
  --exclude='*.zip' \
  --exclude='*.zst' \
  --exclude='*.patch' \
  --exclude='*.diff' \
  --exclude='.DS_Store' \
  --exclude='*/.DS_Store' \
  --exclude='Thumbs.db' \
  --exclude='*/Thumbs.db' \
  -cf "$OUT" \
  .

echo "Wrote $OUT"
