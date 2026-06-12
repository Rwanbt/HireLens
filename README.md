# HireLens

HireLens is a production-oriented CLI for CV optimization. The binary is named `adaptai`.

Core safety rule: LLMs only return structured JSON. Rust validates every adaptation and renders the final CV.

## Commands

```sh
adaptai audit examples/cv.md examples/job.txt --offline
adaptai audit examples/cv.md examples/job.txt --offline --json
adaptai audit examples/cv.md examples/job.txt --offline --min-score 70
adaptai adapt examples/cv.md examples/job.txt --offline --output optimized-cv.md
adaptai adapt examples/cv.md examples/job.txt --offline --diff --min-score 60
adaptai build examples/cv.md --output cv.md
```

`audit` prints a human-readable report by default. Use `--json` for automation.
`adapt` writes Markdown rendered by Rust only; `--diff` shows how the rendered CV
differs from the original Markdown.

Provider selection:

```sh
adaptai audit cv.md job.txt --provider openai
adaptai audit cv.md job.txt --provider ollama
adaptai audit cv.md job.txt --provider lmstudio
```

OpenAI reads `OPENAI_API_KEY`, then `$HIRELENS_CONFIG`, then the OS config file
`hirelens/config.toml`.

Configuration can also live in `./hirelens.toml`. Start from:

```sh
copy hirelens.example.toml hirelens.toml
```

Example:

```toml
provider = "ollama"
offline = false
cache = true
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

Environment variables such as `OPENAI_API_KEY`, `OPENAI_MODEL`,
`OLLAMA_MODEL`, and `LMSTUDIO_MODEL` override provider-specific config.

Local defaults:

- Ollama: `http://localhost:11434`, model `llama3.1`
- LM Studio: `http://localhost:1234/v1`, model `local-model`

Override with `OLLAMA_BASE_URL`, `OLLAMA_MODEL`, `LMSTUDIO_BASE_URL`, or `LMSTUDIO_MODEL`.
