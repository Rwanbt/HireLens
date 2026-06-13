<div align="center">
  <img src="Banner_HireLens.png" alt="HireLens Banner" width="100%" />
</div>

<br />

<div align="center">

  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](LICENSE)
  [![Rust Edition](https://img.shields.io/badge/Rust-2021-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
  [![Build](https://img.shields.io/badge/build-passing-brightgreen?style=for-the-badge)](#)
  [![Tests](https://img.shields.io/badge/tests-17%20passed-brightgreen?style=for-the-badge)](#)
  [![Offline](https://img.shields.io/badge/offline-ready-blue?style=for-the-badge)](#)
  [![Status](https://img.shields.io/badge/status-work%20in%20progress-orange?style=for-the-badge)](#)

</div>

<br />

<div align="center">

  **[ 🇬🇧 English &nbsp;|&nbsp; 🇫🇷 [Lire en Français](README.fr.md) ]**

</div>

---

> ⚠️ **This project is under active development. Expect breaking changes.**

### What is HireLens?

**HireLens** is a production-grade CLI tool that analyzes CVs against job descriptions using ATS scoring and AI assistance — and produces optimized CVs **without hallucinations**.

The core principle: **LLMs only return structured JSON. Rust validates every adaptation and renders the final CV.** No AI-invented skills. No fabricated experience. Ever.

---

### ✨ Features

| Feature | Description |
|---|---|
| 🎯 **ATS Scoring** | HashSet-based skill matching with a normalized 0–100 score |
| 🤖 **Multi-Provider LLM** | OpenAI, Ollama, LM Studio — switch with one flag |
| 🔒 **Anti-Hallucination** | Every adapted skill and bullet is validated against the original CV |
| 📴 **Offline Mode** | Full audit and adaptation without any LLM call |
| 💾 **Smart Cache** | SHA-256 hashed LLM responses stored in `.cache/` |
| 🔏 **Privacy-First** | Local-only mode with Ollama or LM Studio |
| 📄 **Clean Export** | Markdown output rendered by Rust templates; optional PDF via Pandoc |
| 📊 **JSON Output** | Machine-readable `--json` flag for CI/CD pipelines |

---

### 🏗 Architecture

```
src/
├── cli/        # clap-based commands: audit, adapt, build
├── llm/        # LLM trait + OpenAI / Ollama / LM Studio providers
├── core/       # ATS scoring, skill extraction, validation, pipeline
├── parser/     # Markdown + YAML frontmatter CV parser
├── export/     # Rust template renderer + Pandoc PDF bridge
└── utils/      # Config loader (TOML + env), SHA-256 cache
```

**Pipeline:**

```
CV (Markdown+YAML) ──► Parse ──► Extract skills (LLM → JSON)
                                        │
Job description ────► Parse ──► ATS Score (Rust)
                                        │
                              Generate adaptation (LLM → JSON)
                                        │
                              Validate (no new skills allowed) ◄── REJECT if hallucinated
                                        │
                              Render final CV (Rust template)
                                        │
                              Markdown / PDF output
```

---

### 🚀 Installation

**Prerequisites:** [Rust toolchain](https://rustup.rs/) 1.75+

```bash
git clone https://github.com/Rwanbt/HireLens.git
cd HireLens
cargo build --release
# Binary: ./target/release/hirelens
```

Add to your PATH:
```bash
# Linux / macOS
export PATH="$PATH:$(pwd)/target/release"

# Windows (PowerShell)
$env:PATH += ";$(pwd)\target\release"
```

---

### 📖 Usage

#### `audit` — ATS analysis

```bash
# Human-readable report (offline, no LLM needed)
hirelens audit examples/cv.md examples/job.txt --offline

# JSON output for CI/CD
hirelens audit examples/cv.md examples/job.txt --offline --json

# Fail if score is below threshold
hirelens audit examples/cv.md examples/job.txt --offline --min-score 70
```

**Example output:**
```
ATS audit

Score: 63/100
Skill match: 62%

Matched skills: docker, kubernetes, postgresql, rust, tokio
Missing skills: ci/cd, llm, rest
```

#### `adapt` — Optimized CV generation

```bash
# Adapt with offline extraction
hirelens adapt examples/cv.md examples/job.txt --offline --output optimized-cv.md

# Show diff between original and adapted CV
hirelens adapt examples/cv.md examples/job.txt --offline --diff --min-score 60

# Use a cloud LLM
hirelens adapt examples/cv.md examples/job.txt --provider openai --output optimized-cv.md
```

#### `build` — Clean CV rendering

```bash
# Render to Markdown
hirelens build examples/cv.md --output cv.md

# Render to PDF (requires Pandoc)
hirelens build examples/cv.md --output cv.pdf --pdf
```

#### `gui` — Graphical interface

```bash
hirelens gui
```

---

### 🤖 LLM Providers

| Provider | Flag | Default URL | Auth |
|---|---|---|---|
| **OpenAI** | `--provider openai` | `https://api.openai.com/v1` | `OPENAI_API_KEY` env var |
| **Ollama** | `--provider ollama` | `http://localhost:11434` | None |
| **LM Studio** | `--provider lmstudio` | `http://localhost:1234/v1` | None |
| **Gemini** | GUI only | `https://generativelanguage.googleapis.com` | OAuth2 PKCE (⚙️ Settings panel) |

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."
hirelens audit cv.md job.txt --provider openai

# Ollama (requires Ollama running locally)
hirelens audit cv.md job.txt --provider ollama

# LM Studio (requires LM Studio server running)
hirelens audit cv.md job.txt --provider lmstudio
```

---

### ⚙️ Configuration

Copy the example config and edit:

```bash
cp hirelens.example.toml hirelens.toml
```

```toml
# hirelens.toml
provider = "ollama"       # default provider
offline = false           # privacy-first offline mode
cache = true              # cache LLM responses
cache_dir = ".cache"
timeout_seconds = 60

[openai]
model = "gpt-4o-mini"
base_url = "https://api.openai.com/v1"

[ollama]
model = "llama3.1"
base_url = "http://localhost:11434"

[lmstudio]
model = "local-model"
base_url = "http://localhost:1234/v1"
```

**Environment variable overrides:**

| Variable | Description |
|---|---|
| `OPENAI_API_KEY` | OpenAI API key |
| `OPENAI_MODEL` | Override OpenAI model |
| `OLLAMA_MODEL` | Override Ollama model |
| `OLLAMA_BASE_URL` | Override Ollama URL |
| `LMSTUDIO_MODEL` | Override LM Studio model |
| `LMSTUDIO_BASE_URL` | Override LM Studio URL |
| `HIRELENS_CONFIG` | Path to custom config file |

---

### 🔒 Anti-Hallucination System

HireLens enforces strict rules at every stage:

1. **JSON-only LLM output** — the model is constrained to return structured JSON, never free-form text
2. **Skill whitelist** — every skill in the adapted output must exist in the original CV
3. **Bullet validation** — every adapted bullet must be traceable to an original bullet
4. **Rust rendering** — the final CV text is assembled by Rust templates, not by the LLM
5. **Diff visibility** — `--diff` flag exposes every change between original and adapted output

```
Original CV skills: [Rust, Docker, Kubernetes, PostgreSQL]
LLM proposes:       [Rust, Docker, Kubernetes, PostgreSQL, Go]  ← REJECTED
Validated output:   [Rust, Docker, Kubernetes, PostgreSQL]      ✓
```

---

### 🧪 Tests

```bash
cargo test
# 17 tests passed — cli, llm, core, parser, export, utils
```

---

### 📋 CV Format

HireLens expects a Markdown file with a YAML frontmatter block:

```markdown
---
name: Jane Doe
headline: Senior Backend Engineer
summary: Systems engineer focused on reliable distributed systems.
skills:
  - Rust
  - Docker
  - Kubernetes
experience:
  - id: exp-1
    company: Acme Corp
    role: Backend Engineer
    start: "2020"
    end: Present
    bullets:
      - Built microservices with Rust and Tokio.
education:
  - institution: MIT
    degree: B.S. Computer Science
    year: "2018"
---
```

---

### 📄 License

[MIT](LICENSE) — © 2026 HireLens

---

<div align="right">

**[🔝 Back to top](#)**

</div>
