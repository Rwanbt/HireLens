@AGENTS.md

# HireLens — Claude Code Guidelines

## Qu'est-ce que HireLens

CLI + GUI Rust d'optimisation de CV avec scoring ATS et **validation anti-hallucination stricte**.
Les LLMs ne retournent que du JSON structuré. Rust valide chaque compétence et bullet contre le CV
original. Aucun contenu inventé ne peut atteindre la sortie.

Repo public : https://github.com/Rwanbt/HireLens — MIT

## Build

```powershell
# Build release
cargo build --release
# Binary : target/release/hirelens.exe

# Build debug
cargo build

# Run GUI
cargo run -- gui

# Run CLI audit (offline)
cargo run -- audit examples/cv.md examples/job.txt --offline
```

## Invariant critique — Anti-Hallucination (NE JAMAIS AFFAIBLIR)

`src/core/validation.rs` est la **frontière de sécurité centrale** du produit.

Règles absolues :
- Tout skill proposé par un LLM **doit exister** dans `cv.skills` (normalisé)
- Tout bullet adapté **doit exister verbatim** dans `cv.experience[*].bullets`
- Toute modification qui assouplit ces règles **nécessite un ADR explicite** et une discussion

```
LLM → JSON structuré → validate_adaptation() → REJECT si hallucination → Renderer Rust
                              ↑ NE PAS CONTOURNER
```

## Moteur offline — matcher déterministe (NE JAMAIS Y INTRODUIRE DE GÉNÉRATION)

`src/core/offline_match.rs` est un moteur **100 % algorithmique** (cf. ADR-0008).
Il ne fait que **sélectionner et classer de l'existant** — l'anti-hallucination y
est gratuite par construction.

Règles absolues :
- Les bullets sont sélectionnés **par index** (`Vec<usize>`) et copiés **verbatim** ;
  jamais de réécriture, fusion ou concaténation.
- `prioritized_skills` est toujours un **ré-ordonnancement** de l'allowed-set, jamais
  une invention.
- `core/` ne dépend **jamais** de `llm/` (la bifurcation et le mapping DTO vivent
  dans `pipeline.rs`).
- Toute introduction de génération (paraphrase, synthèse, complétion) y est
  **interdite** sans ADR explicite.

## LLM Providers — Conventions

Ajouter un nouveau provider :
1. Créer `src/llm/<name>.rs` — implémenter `trait LlmProvider`
2. Ajouter variant dans `LlmProviderKind` (`src/llm/provider.rs`)
3. Câbler dans `LlmRouter::new()` (CLI) et `from_gui()` (GUI) dans `src/llm/router.rs`
4. Ajouter une entrée dans la table README + la config exemple `hirelens.example.toml`

Providers actuels : OpenAI (env/keyring) · Ollama (local) · LM Studio (local) · Gemini (GUI OAuth2 PKCE)

> **Note** : Gemini est GUI-only (OAuth2 PKCE via `src/auth/`). Appeler `LlmRouter::from_gui()`,
> pas `LlmRouter::new()`. `LlmRouter::new(Gemini)` retourne une erreur explicite.

## GUI — Architecture (après refactor e8ee207)

```
src/gui/
├── mod.rs          ← run(), constantes COL_* (ne pas dupliquer ailleurs)
├── app.rs          ← HireLensApp — impl eframe::App, update()
├── state.rs        ← AppState — état partagé, sans logique GUI
├── views/          ← Écrans principaux (main_view, settings_view)
├── widgets/        ← Widgets réutilisables (chips, gauge — réutiliser avant d'en créer)
├── settings.rs     ← Panneau settings (OAuth2 Gemini, clés API, URLs)
└── html_export.rs  ← Export HTML maison
```

Règles GUI :
- Les couleurs constantes sont dans `gui/mod.rs` — `COL_GREEN`, `COL_RED`, `COL_YELLOW`, etc.
- Toute logique métier reste dans `core/` — `gui/` ne fait qu'appeler et afficher
- `state.rs` contient l'état ; `app.rs` orchestre — ne pas mélanger

## Pre-commit Checklist

Avant chaque `git commit` touchant du code :

```powershell
# Lint + tests
cargo clippy --all-targets -- -D warnings
cargo test

# Audit sécurité (quand cargo-audit est installé)
cargo audit
```

Si l'un échoue → ne pas committer. Régler d'abord.

## Health Stack

```powershell
# Tests
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Typecheck (équivalent)
cargo check

# Format check
cargo fmt --check
```

## Qualité — Règles Rust

- **Zéro `unwrap()` en code de production** — utiliser `?`, `.map_err()`, ou `anyhow::bail!`
  Chaque `unwrap()` restant doit avoir `// SAFETY: [raison prouvée]`
- **Zéro warning** — `cargo clippy -D warnings` doit passer au vert
- **Fonctions ≤ 80 LOC** — au-delà : extraire une sous-fonction avant d'ajouter du code
- **Gestion d'erreurs aux frontières** : I/O, HTTP, parsing utilisateur → toujours `Result<T, E>`

## Conventions de Nommage (Rust)

| Catégorie | Convention | Exemple |
|---|---|---|
| Types, Enums, Traits | PascalCase | `LlmProvider`, `AdaptationResponse`, `AppState` |
| Fonctions, méthodes | snake_case | `validate_adaptation()`, `extract_skills()` |
| Variables, membres | snake_case | `provider_kind`, `cache_dir` |
| Constantes | SCREAMING_SNAKE_CASE | `COL_GREEN`, `MAX_RETRY_COUNT` |
| Modules | snake_case | `llm`, `core`, `auth` |

## Git Conventions

- Format commit : `<type>(<scope>): <description>` — feat, fix, refactor, perf, docs, test, chore
- Exemples : `feat(llm): add Anthropic provider`, `fix(validation): reject empty skill strings`
- Ne jamais force-push sur `master`

## Infrastructure (livrée ✅)

| Item | Statut | Description |
|------|--------|-------------|
| `.github/workflows/ci.yml` | ✅ | fmt + clippy -D + test + cargo-deny + cargo-audit |
| `deny.toml` (cargo-deny) | ✅ | licenses allowlist (MIT/Apache-2) + bans + CVE scan |
| `docs/adr/` (ADR-0001 → ADR-0008) | ✅ | Anti-hal, LLM trait, egui, Gemini OAuth2, PDF, cache, auth, offline |
| `ARCHITECTURE.md` | ✅ | Thread model, pipeline anti-hal, ownership modules |
| README : table providers | ✅ | Gemini GUI-only (OAuth2 PKCE) documenté |

## ADRs — Décisions documentées

| ADR | Sujet |
|-----|-------|
| **ADR-0001** | Anti-hallucination via validation Rust post-LLM |
| **ADR-0002** | `trait LlmProvider` + router multi-provider |
| **ADR-0003** | egui/eframe 0.29 pour la GUI |
| **ADR-0004** | Gemini GUI-only via OAuth2 PKCE |
| **ADR-0005** | PDF renderer trait |
| **ADR-0006** | Word boundary + provider cache key |
| **ADR-0007** | Gemini OAuth embarqué + clé API par utilisateur |
| **ADR-0008** | Moteur offline déterministe (zéro génération, sélection par index) |
