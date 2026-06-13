#!/usr/bin/env bash
# run_hook.sh — Claude Code PostToolUse hook wrapper.
# Reads stdin once, finds a working Python, delegates to update_on_edit.py.
# Always exits 0 so it never blocks Claude Code.

INPUT=$(cat)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$SCRIPT_DIR/update_on_edit.py"

PY=$(bash "$SCRIPT_DIR/find_python.sh")
if [ -n "$PY" ]; then
    echo "$INPUT" | PYTHONIOENCODING=utf-8 "$PY" "$SCRIPT" 2>&1
fi

# No working Python found — silent skip (do not block Claude Code)
exit 0
