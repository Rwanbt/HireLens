#!/usr/bin/env bash
# find_python.sh — Emit the path to a working Python 3 interpreter.
# Honors PYTHON_BIN from config.sh, then tests generic candidates with a real
# run to skip MS Store stubs. Usage: PY=$(bash tools/ai_docs/find_python.sh)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
[ -f "$SCRIPT_DIR/config.sh" ] && source "$SCRIPT_DIR/config.sh"

CANDIDATES=(
    "${PYTHON_BIN:-}"
    "python3"
    "python"
    "py"
)

for PY in "${CANDIDATES[@]}"; do
    [ -z "$PY" ] && continue
    if "$PY" -c "import sys; sys.exit(0)" >/dev/null 2>&1; then
        echo "$PY"
        exit 0
    fi
done

echo ""
exit 1
