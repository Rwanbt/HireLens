# HireLens — Roadmap & Plan

> **Source de vérité unique** du plan produit + technique. Plan stratégique *et* todolist.
> Mettre à jour ce fichier à chaque fin de session : cocher les `[x]`, ajuster les priorités.
>
> Dernière mise à jour : **2026-06-13**

---

## 🎯 Vision

HireLens n'est ni un éditeur de CV, ni un simple ATS checker. C'est :

> **Un agent IA local qui comprend les systèmes de recrutement et réécrit des documents de carrière sans jamais halluciner.**

Trois principes non négociables :

1. **Anti-hallucination** — les LLMs ne proposent que du JSON ; Rust valide et assemble. `validate_adaptation()` est la frontière de sécurité (voir [ADR-0001](adr/0001-anti-hallucination-validation.md)).
2. **Local-first / privacy-first** — Ollama + LM Studio sont les providers prioritaires. Le cloud (OpenAI, Gemini OAuth2) est secondaire et **non prioritaire**.
3. **Recruteur-ready en 1 clic** — export PDF/HTML pro, offline, sans dépendance externe.

---

## 📊 Légende des statuts

| Marqueur | Sens |
|----------|------|
| `[x]` | Fait et vérifié (build + test) |
| `[ ]` | À faire |
| `[~]` | En cours |
| 🧊 | Icebox — décidé volontairement *plus tard* (justifié en bas) |

---

## ✅ Déjà fait (snapshot)

- [x] CLI `audit` / `adapt` / `build` (clap, offline + providers)
- [x] Pipeline anti-hallucination (`core/pipeline.rs` + `core/validation.rs`)
- [x] ATS scoring HashSet (`core/ats.rs`)
- [x] 4 providers LLM (OpenAI, Ollama, LM Studio, Gemini) via `trait LlmProvider`
- [x] GUI egui/eframe (refactor submodules : `state` / `views` / `widgets`)
- [x] Settings panel + keyring (secrets OS-level) + Gemini OAuth2 PKCE
- [x] File dialogs (rfd) + export HTML maison + copy-to-clipboard
- [x] Export PDF via Typst (`export/typst_render.rs`) — PR 2.5
- [x] **Mode web `hirelens serve`** (Axum, `/api/audit` + `/api/adapt`, UI single-page) — PR Web
- [x] Infra : CI GitHub Actions, `deny.toml`, ADRs 0001–0004, `ARCHITECTURE.md`
- [x] Stack `.claude/` (agents, hooks PostToolUse, AI_CONTEXT.md par module)

---

## 🔥 Phase 1 — Feature killer : "Explain why score is X"

> **Priorité absolue.** C'est ce qui transforme un score opaque en produit intelligent.
> Aujourd'hui le score n'est qu'un ratio `matched/total`. Aucune explication.

### 1.1 — Moteur de matching enrichi (`core/matching/`)
- [ ] Créer `core/matching/mod.rs` — extraire la logique de `ats.rs`
- [ ] `keyword_match.rs` : comptage d'**occurrences** de chaque skill dans le texte de l'offre (pas juste présence/absence)
- [ ] `gap_analysis.rs` : pour chaque skill manquant → nombre d'occurrences dans l'offre + poids (skill cité 3× = critique)
- [ ] Pondérer le score : un skill manquant cité 3× pèse plus qu'un cité 1×
- [ ] Tests unitaires sur le comptage d'occurrences et la pondération

### 1.2 — Structure d'explication
- [ ] Étendre `AuditReport` avec un champ `explanations: Vec<ScoreReason>`
- [ ] `ScoreReason { skill, status (Present/Missing/Weak), occurrences, weight }`
- [ ] Garder la rétro-compat JSON (`--json`) — versionner si breaking

### 1.3 — Rendu de l'explication (3 surfaces)
- [ ] CLI : section `Why:` sous le score dans `format_audit_report()`
- [ ] GUI : panneau dépliable "Pourquoi ce score ?" dans `main_view.rs`
- [ ] Web : bloc explication dans `ui.html`

**Exemple cible :**
```
Score: 72/100

Why:
- Rust         ✔ present
- Docker       ❌ missing (3 occurrences in job — critical)
- CI/CD        ⚠ weak (mentioned once, not in your skills)
```

---

## 🏗 Phase 2 — Durcissement architecture

> Réduire la fragilité avant de scaler. Petits pas réversibles, pas de big-bang restructure.

### 2.1 — Abstraction `PdfRenderer` (découpler de Typst)
- [ ] Définir `trait PdfRenderer { fn render(&self, cv: &str) -> Result<Vec<u8>>; }`
- [ ] `TypstRenderer` implémente le trait (déplacer `export_pdf` dedans)
- [ ] L'appelant (GUI `start_export_pdf`) dépend du trait, pas de Typst
- [ ] ADR-0005 : choix Typst + porte de sortie (fallback futur)

### 2.2 — Layer Controller GUI (anti god-view)
- [ ] Créer `gui/controller.rs` — orchestration UI → core
- [ ] Déplacer `start_audit` / `start_adapt` / `start_export_*` de `app.rs` vers le controller
- [ ] Règle : `views/` = render only · `controller` = actions · `core/` = intelligence
- [ ] Vérifier que `app.rs` retombe sous 300 LOC après extraction

### 2.3 — Fallback LlmRouter local (optionnel, local-first)
- [ ] Si Ollama indisponible → tenter LM Studio → sinon offline
- [ ] Message clair à l'utilisateur sur le provider effectivement utilisé
- [ ] ⚠️ **Pas** de fallback vers cloud (respect du local-first)

---

## 🧹 Phase 3 — Polish & infra

- [ ] Supprimer / gitignore les artefacts test (`test-build.md`, `test-build-pdf.md`, `test-optimized.md`, `optimized-cv.md`)
- [ ] `docs/KNOWN_FAILURE_PATTERNS.md` (signalé manquant par `docs/METRICS.md`)
- [ ] `AI_CONTEXT.md` manquants sur `src/` racine et `src/gui/` (risk zones du METRICS)
- [ ] README : documenter `hirelens serve` dans la section Usage
- [ ] README badge tests : passer de "17 passed" → count réel (24)
- [ ] `cargo install cargo-deny` localement + tester `deny.toml` avant push
- [ ] `docs/GOOGLE_OAUTH_SETUP.md` — guide Google Cloud (⬇️ **dé-priorisé** : cloud non prioritaire)

---

## 🧊 Icebox — volontairement reporté

Suggéré par la review ChatGPT mais **non retenu maintenant**, avec justification :

| Suggestion | Pourquoi reporté |
|------------|------------------|
| **Action System** (`enum Action { RunAudit, ... }`) | Utile seulement pour un agent UI futur. Spéculatif tant qu'on n'a pas d'agent. À reconsidérer si on ajoute le pilotage IA de l'UI. |
| **Restructure complète** (`providers/`, `storage/`, `actions/`, renommage `llm/`) | Énorme churn pour bénéfice marginal. La structure actuelle (`core/gui/export/llm/auth/parser/utils/web`) est déjà propre. Viole "petits pas réversibles". On ajoute du ciblé (`core/matching/`, `gui/controller.rs`) au lieu de tout déplacer. |
| **`core/async_runtime.rs` global unique** | `std::thread::spawn` + tokio par appel fonctionne. Pas de douleur mesurée. À faire si on observe des bugs de threading réels. |
| **Semantic match (embeddings)** | Le keyword match enrichi (Phase 1) couvre 80% du besoin offline. Les embeddings ajoutent une dépendance lourde. À évaluer après Phase 1. |

---

## 📌 Prochaine action

➡️ **Phase 1.1** — créer `core/matching/` avec le comptage d'occurrences. C'est la fondation du "Explain why".
