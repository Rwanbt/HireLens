# ARCHITECTURE.md — HireLens

## Vue d'ensemble

HireLens est un binaire Rust unique avec deux modes d'exécution partageant la même logique métier :

```
hirelens gui     → egui event loop (eframe)
hirelens audit   → tokio multi-thread runtime (CLI)
hirelens adapt   → tokio multi-thread runtime (CLI)
hirelens build   → tokio multi-thread runtime (CLI)
```

---

## Thread Model

### Mode CLI

```
main() — tokio multi-thread runtime
    └── cli::run()
            ├── Parser (sync) → Cv + JobDescription
            ├── LlmRouter::new() (sync)
            └── Pipeline::audit_text() / adapt_text() (async, tokio)
                    └── LlmProvider::extract_skills() / generate_adaptation() (reqwest async)
```

Le runtime tokio multi-thread est créé dans `main()` via `Builder::new_multi_thread()`.

### Mode GUI

```
main() → gui::run() → eframe::run_native() [egui event loop — main thread]
    │
    └── HireLensApp::update() — appelé ~60fps par eframe
            │
            ├── start_audit() / start_adapt() → std::thread::spawn()
            │       └── tokio::runtime::Builder::new_current_thread()
            │               └── LlmRouter::from_gui() + Pipeline [async]
            │                       └── résultat via mpsc::channel → poll_results()
            │
            ├── start_open_file() / start_save_md() → std::thread::spawn()
            │       └── rfd::FileDialog (bloque le thread spawné, jamais le GUI)
            │               └── résultat via mpsc::channel → poll_results()
            │
            └── start_google_auth() → std::thread::spawn()
                    └── auth::start_google_oauth_sync() (sync wrapper)
                            └── résultat via mpsc::channel → poll_results()
```

**Règle critique** : `HireLensApp::update()` ne doit jamais bloquer. Toute opération async ou
bloquante passe par `std::thread::spawn()` + `mpsc::channel` + `ctx.request_repaint()`.

---

## Pipeline Anti-Hallucination (invariant central)

```
Input
  CV (Markdown + YAML frontmatter)
  Job Description (texte libre)
        │
        ▼
   parser::parse_cv() + parser::parse_job()
        │
        ▼ (si !offline)
   LlmProvider::extract_skills() → JSON uniquement
        │
        ▼
   core::ats::compute_audit() — Rust pur, zéro LLM
        │
        ▼ (si adapt)
   LlmProvider::generate_adaptation() → JSON uniquement
        │
        ▼
   ╔═══════════════════════════════════════╗
   ║  core::validation::validate_adaptation() ║  ← FRONTIÈRE DE SÉCURITÉ
   ║  • skill ∈ cv.skills (normalisé)       ║
   ║  • bullet ∈ cv.experience[*].bullets   ║
   ║  • REJECT si violation                 ║
   ╚═══════════════════════════════════════╝
        │
        ▼
   export::render_markdown() — Rust pur, zéro LLM
        │
        ▼
   Markdown / PDF / HTML output
```

`validate_adaptation()` dans `src/core/validation.rs` est la **seule barrière** entre les
propositions LLM et la sortie utilisateur. Elle ne peut pas être contournée ou assouplie sans ADR.

---

## Ownership des Modules

```
src/
├── main.rs          Dispatch GUI vs CLI ; init runtime
├── cli/             Commands clap — dépend de core/ + llm/ + parser/ + export/
├── llm/             Abstraction LLM — dépend de core/ (types Cv, JobDescription)
│   ├── provider.rs  trait LlmProvider + types (pas de deps montantes)
│   ├── router.rs    LlmRouter — dépend de llm/*providers + auth/
│   └── *provider    Implémentations — dépend de provider.rs uniquement
├── core/            Logique métier pure — ZÉRO dépendance sur gui/ ou cli/
│   ├── mod.rs       Types Cv, Experience, JobDescription
│   ├── ats.rs       compute_audit() — pur, testable sans réseau
│   ├── skills.rs    normalize_skill(), skill_set()
│   ├── validation.rs validate_adaptation() — FRONTIÈRE CRITIQUE
│   └── pipeline.rs  Orchestration — dépend de llm/ + core/*
├── parser/          Markdown/YAML → Cv — dépend de core/
├── export/          Cv → Markdown/PDF — dépend de core/
├── auth/            OAuth2 PKCE Google — dépend de rien de core/
│   ├── google.rs    Flux OAuth2 complet
│   ├── pkce.rs      Génération PKCE (code_verifier, code_challenge)
│   ├── oauth_server.rs Serveur local pour redirect_uri (port 8080)
│   └── token_store.rs  keyring + sérialisation token
├── gui/             egui/eframe — dépend de core/ + llm/ + auth/ + parser/ + export/
│   ├── mod.rs       run(), constantes couleur
│   ├── app.rs       HireLensApp — état + opérations async
│   ├── state.rs     AppState (AuditState, AdaptState)
│   ├── views/       Rendus par écran
│   ├── widgets/     Composants réutilisables
│   ├── settings.rs  Panneau settings + persistance
│   └── html_export.rs
└── utils/           Config TOML + cache SHA-256 — dépend de rien d'interne
```

**Direction de dépendance** : `gui/` → `core/` → `types`. Jamais `core/` → `gui/`.

---

## Cache LLM

Les réponses LLM sont cachées dans `.cache/` avec une clé SHA-256 de l'input. Le cache est
content-addressed : même input → même fichier. Désactivable avec `--no-cache` ou `cache = false`
dans `hirelens.toml`.

Format : `.cache/<sha256hex>.json`

---

## Auth Google Gemini (OAuth2 PKCE)

Gemini est le seul provider qui n'utilise pas de clé API statique. Le flux :

```
Settings panel → start_google_auth()
    → std::thread::spawn
        → auth::start_google_oauth_sync()
            → pkce::generate() (code_verifier + code_challenge)
            → ouvre le navigateur système (google.com/auth)
            → oauth_server: serveur HTTP local port 8080 attend le redirect
            → échange code contre access_token + refresh_token
            → token_store: stocke dans keyring OS
```

À chaque appel LLM Gemini : `auth::get_valid_access_token()` vérifie l'expiration et rafraîchit
automatiquement via le refresh_token.

---

## Décisions architecturales

Voir `docs/adr/` pour les ADRs détaillés :

- [ADR-0001](docs/adr/0001-anti-hallucination-validation.md) — Validation anti-hallucination post-LLM
- [ADR-0002](docs/adr/0002-llm-provider-trait.md) — `trait LlmProvider` + router multi-provider
- [ADR-0003](docs/adr/0003-egui-eframe.md) — egui/eframe pour la GUI
- [ADR-0004](docs/adr/0004-gemini-oauth2-gui-only.md) — Gemini GUI-only via OAuth2 PKCE
