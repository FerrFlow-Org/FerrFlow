#!/usr/bin/env bash
set -euo pipefail

# Generate shell completion scripts for ferrflow.
# Usage: ./scripts/generate-completions.sh [output_dir]

FERRFLOW="${FERRFLOW_BIN:-ferrflow}"
OUT_DIR="${1:-completions}"

mkdir -p "$OUT_DIR"

"$FERRFLOW" completions bash > "$OUT_DIR/ferrflow.bash"
"$FERRFLOW" completions zsh  > "$OUT_DIR/_ferrflow"
"$FERRFLOW" completions fish > "$OUT_DIR/ferrflow.fish"

echo "Generated completions in $OUT_DIR/"
ls -1 "$OUT_DIR/"
