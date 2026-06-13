#!/usr/bin/env bash
# config.sh — Machine-specific paths for the AI docs stack (HIRELENS).
# Defaults match the primary dev machine; override via environment if needed.
# Sourced by run_hook.sh, find_python.sh, and the verify-ai-docs skill.

# ── graphify ──────────────────────────────────────────────────────────────────
GRAPHIFY_BIN="${GRAPHIFY_BIN:-graphify}"

# ── Obsidian Vault ────────────────────────────────────────────────────────────
OBSIDIAN_VAULT="${OBSIDIAN_VAULT:-/d/Documents/Obsidian/IA_Dev_Brain}"
OBSIDIAN_PROJECT_DIR="${OBSIDIAN_PROJECT_DIR:-HireLens}"
OBSIDIAN_MEMORY_FILE="${OBSIDIAN_MEMORY_FILE:-$OBSIDIAN_PROJECT_DIR/_memory/memory.md}"
OBSIDIAN_LOG_FILE="${OBSIDIAN_LOG_FILE:-LOG.md}"

# ── Claude Code Memory ────────────────────────────────────────────────────────
CLAUDE_MEMORY_ROOT="${CLAUDE_MEMORY_ROOT:-$HOME/.claude/projects}"
CLAUDE_MEMORY_KEY="${CLAUDE_MEMORY_KEY:-d--App-HireLens}"

# ── Python ────────────────────────────────────────────────────────────────────
# Populated automatically by find_python.sh — override here if needed
# PYTHON_BIN="/path/to/python"

# ── Skills ────────────────────────────────────────────────────────────────────
SKILLS_DIR="${SKILLS_DIR:-.claude/skills}"
EXPECTED_SKILLS="${EXPECTED_SKILLS:-}"
