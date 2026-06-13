# HireLens — Roadmap & Plan

> **Source de vérité unique** du plan produit + technique. Plan stratégique *et* todolist.
> Mettre à jour ce fichier à chaque fin de session : cocher les `[x]`, ajuster les priorités.
>
> Dernière mise à jour : **2026-06-13**

---

## ⛔ RÈGLES — À LIRE AVANT DE CODER (obligatoire)

**1. La boucle d'or — répéter pour CHAQUE tâche cochée :**

```
1. Lire le fichier indiqué (📁) AVANT de l'éditer.
2. Faire UNE seule tâche à la fois (un ✅ = un changement).
3. Lancer la commande de vérif (✅ Vérifier) → doit passer.
4. Cocher [x] dans CE fichier.
5. Commit : git commit -m "<type>(<scope>): <desc>"
```

**2. Interdictions absolues (ne JAMAIS faire) :**

- ❌ **NE JAMAIS toucher à `src/core/validation.rs`** ni affaiblir `validate_adaptation()`.
  C'est la frontière anti-hallucination. La modifier exige un ADR + accord humain. Voir [ADR-0001](adr/0001-anti-hallucination-validation.md).
- ❌ **NE JAMAIS** ajouter de `.unwrap()` ou `.expect()` en code de production (sauf `// SAFETY: <raison>`).
- ❌ **NE JAMAIS** passer à la tâche suivante si `cargo build` ou `cargo test` échoue. Réparer d'abord.
- ❌ **NE JAMAIS** faire deux phases en même temps. Finir 1.1 avant 1.2.

**3. Les 2 commandes qui valident tout :**

```powershell
cargo build          # doit finir par "Finished" sans erreur
cargo test           # doit finir par "test result: ok"
```

Si l'une échoue → la tâche n'est PAS finie. Ne pas cocher.

**4. En cas de doute → STOP et demander à l'humain.** Ne jamais inventer une API ou un chemin de fichier.

---

## 🎯 Vision

HireLens n'est ni un éditeur de CV, ni un simple ATS checker. C'est :

> **Un agent IA local qui comprend les systèmes de recrutement et réécrit des documents de carrière sans jamais halluciner.**

Trois principes non négociables :

1. **Anti-hallucination** — les LLMs ne proposent que du JSON ; Rust valide et assemble.
2. **Local-first / privacy-first** — Ollama + LM Studio prioritaires. Le cloud (OpenAI, Gemini OAuth2) est **non prioritaire**.
3. **Recruteur-ready en 1 clic** — export PDF/HTML pro, offline.

---

## 📊 Légende des statuts

| Marqueur | Sens |
|----------|------|
| `[x]` | Fait ET vérifié (build + test passent) |
| `[ ]` | À faire |
| `[~]` | En cours (une seule tâche peut être `[~]` à la fois) |
| 🧊 | Icebox — reporté volontairement (justifié en bas) |

---

## ✅ Déjà fait (snapshot — ne pas refaire)

- [x] CLI `audit` / `adapt` / `build`
- [x] Pipeline anti-hallucination (`core/pipeline.rs` + `core/validation.rs`)
- [x] ATS scoring HashSet (`core/ats.rs`)
- [x] 4 providers LLM (OpenAI, Ollama, LM Studio, Gemini) via `trait LlmProvider`
- [x] GUI egui/eframe (submodules `state` / `views` / `widgets`)
- [x] Settings panel + keyring + Gemini OAuth2 PKCE
- [x] File dialogs + export HTML + copy-clipboard
- [x] Export PDF via Typst (`export/typst_render.rs`)
- [x] Mode web `hirelens serve` (Axum + UI single-page)
- [x] Infra : CI, `deny.toml`, ADRs 0001–0004, `ARCHITECTURE.md`, stack `.claude/`

---

## 🔥 Phase 1 — Feature killer : "Explain why score is X"

> **Priorité absolue.** Aujourd'hui le score est un ratio opaque. Objectif : expliquer POURQUOI.
> **Ordre obligatoire : 1.1 → 1.2 → 1.3.** Ne pas commencer 1.2 avant que 1.1 soit `[x]`.

### 1.1 — Moteur de matching (comptage d'occurrences)

- [x] **1.1.1 — Créer le module `matching`**
  - 📁 Fichier : `src/core/matching.rs` (nouveau) + ajouter `pub mod matching;` dans `src/core/mod.rs`
  - 🔧 Action : créer ce contenu exact :
    ```rust
    use crate::core::skills::normalize_skill;
    use crate::core::JobDescription;

    /// Combien de fois un skill apparaît dans le texte brut de l'offre.
    #[derive(Debug, Clone, PartialEq)]
    pub struct SkillSignal {
        pub skill: String,
        pub occurrences: usize,
    }

    /// Pour chaque skill de l'offre, compte ses occurrences dans `job.raw_text`.
    pub fn count_skill_occurrences(job: &JobDescription) -> Vec<SkillSignal> {
        let haystack = job.raw_text.to_lowercase();
        job.skills
            .iter()
            .map(|raw| {
                let skill = normalize_skill(raw);
                let occurrences = if skill.is_empty() {
                    0
                } else {
                    haystack.matches(skill.as_str()).count()
                };
                SkillSignal { skill, occurrences }
            })
            .collect()
    }
    ```
  - ✅ Vérifier : `cargo build` → "Finished" sans erreur.

- [x] **1.1.2 — Tester le comptage**
  - 📁 Fichier : `src/core/matching.rs` (ajouter un `#[cfg(test)] mod tests` à la fin)
  - 🔧 Action : un test qui construit un `JobDescription` avec `raw_text` contenant "rust" 2 fois et vérifie `occurrences == 2`.
  - ✅ Vérifier : `cargo test count` → le test passe.

### 1.2 — Structure d'explication (dépend de 1.1)

- [x] **1.2.1 — Définir `ScoreReason`**
  - 📁 Fichier : `src/core/matching.rs`
  - 🔧 Action : ajouter :
    ```rust
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub enum SkillStatus { Present, Missing, Weak }

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct ScoreReason {
        pub skill: String,
        pub status: SkillStatus,
        pub occurrences: usize,
    }
    ```
  - 🔧 Règle de statut : `Present` si le skill est dans le CV. Sinon `Missing` si occurrences ≥ 2, sinon `Weak`.
  - ✅ Vérifier : `cargo build`.

- [x] **1.2.2 — Brancher dans `AuditReport`**
  - 📁 Fichier : `src/core/ats.rs`
  - 🔧 Action : ajouter le champ `pub explanations: Vec<crate::core::matching::ScoreReason>` à `AuditReport`, et le remplir dans `compute_audit()` à partir de `count_skill_occurrences()`.
  - ⚠️ Tout endroit qui construit un `AuditReport` à la main (chercher avec : `grep -rn "AuditReport {"`) devra ajouter `explanations: vec![]` ou la vraie valeur, sinon ça ne compile pas.
  - ✅ Vérifier : `cargo build` ET `cargo test` → tout passe.

### 1.3 — Rendu de l'explication, 3 surfaces (dépend de 1.2)

- [x] **1.3.1 — CLI**
  - 📁 Fichier : `src/cli/mod.rs`, fonction `format_audit_report()`
  - 🔧 Action : ajouter une section `Why:` qui liste chaque `ScoreReason` (skill + statut + occurrences).
  - ✅ Vérifier : `cargo run -- audit examples/cv.md examples/job.txt --offline` → la section "Why:" apparaît.

- [x] **1.3.2 — GUI**
  - 📁 Fichier : `src/gui/views/main_view.rs`, fonction `render_audit_panel()`
  - 🔧 Action : ajouter un `egui::CollapsingHeader` "Pourquoi ce score ?" qui liste les `explanations`.
  - ✅ Vérifier : `cargo build` (le rendu visuel se teste à la main avec `cargo run -- gui`).

- [x] **1.3.3 — Web**
  - 📁 Fichier : `src/web/ui.html` (bloc `renderAudit`) + `src/web/mod.rs` (struct `AuditData` doit exposer `explanations`)
  - 🔧 Action : afficher la liste des raisons sous le score dans l'UI web.
  - ✅ Vérifier : `cargo run -- serve --open` → ouvrir, analyser, voir le bloc.

**Exemple cible (sortie CLI) :**
```
Score: 72/100

Why:
- Rust         present
- Docker       missing (3 occurrences in job)
- CI/CD        weak (1 occurrence)
```

---

## 🏗 Phase 2 — Durcissement architecture

> Réduire la fragilité. Petits pas réversibles. **Ordre : 2.1 → 2.2 → 2.3.**

### 2.1 — Abstraction `PdfRenderer`

- [x] **2.1.1 — Définir le trait**
  - 📁 Fichier : `src/export/mod.rs`
  - 🔧 Action : `pub trait PdfRenderer { fn render(&self, markdown: &str) -> anyhow::Result<Vec<u8>>; }`
  - ✅ Vérifier : `cargo build`.

- [x] **2.1.2 — Implémenter pour Typst**
  - 📁 Fichier : `src/export/typst_render.rs`
  - 🔧 Action : créer `pub struct TypstRenderer;` qui implémente `PdfRenderer` en appelant la logique de `export_pdf()` existante.
  - ✅ Vérifier : `cargo test` (les 4 tests typst passent toujours).

- [x] **2.1.3 — Brancher l'appelant sur le trait**
  - 📁 Fichier : `src/gui/app.rs`, méthode `start_export_pdf()`
  - 🔧 Action : appeler `TypstRenderer.render(&markdown)` au lieu de `export_pdf(&markdown)` directement.
  - ✅ Vérifier : `cargo build` + test manuel du bouton PDF.

- [x] **2.1.4 — ADR-0005**
  - 📁 Fichier : `docs/adr/0005-pdf-renderer-trait.md` (nouveau)
  - 🔧 Action : documenter le choix Typst + porte de sortie (fallback futur via le trait).

### 2.2 — Layer Controller GUI

- [x] **2.2.1 — Créer le controller**
  - 📁 Fichier : `src/gui/controller.rs` (nouveau) + `pub mod controller;` dans `src/gui/mod.rs`
  - 🔧 Action : y déplacer `start_audit` / `start_adapt` / `start_export_html` / `start_export_pdf` / `start_save_md` depuis `app.rs`.
  - ⚠️ Déplacer une méthode à la fois, `cargo build` entre chaque.
  - ✅ Vérifier : `cargo build` + `cargo test` après CHAQUE méthode déplacée.

- [x] **2.2.2 — Vérifier la taille d'`app.rs`**
  - ✅ Vérifier : `app.rs` doit retomber sous 300 LOC. Commande : `(Get-Content src/gui/app.rs).Count`.

### 2.3 — Fallback LlmRouter local

- [x] **2.3.1 — Fallback Ollama → LM Studio → offline**
  - 📁 Fichier : `src/llm/router.rs`
  - 🔧 Action : si le provider local échoue à se connecter, essayer le suivant. ⚠️ **JAMAIS** de fallback vers le cloud.
  - ✅ Vérifier : `cargo test`.

---

## 🧹 Phase 3 — Polish & infra (tâches indépendantes, ordre libre)

- [x] **3.1** — Supprimer les artefacts test
  - 🔧 Action : `git rm --cached` ou supprimer `test-build.md`, `test-build-pdf.md`, `test-optimized.md`, `optimized-cv.md`, puis les ajouter à `.gitignore`.
- [x] **3.2** — README : documenter `hirelens serve` dans la section Usage.
  - 📁 Fichier : `README.md` + `README.fr.md`
- [x] **3.3** — README : corriger le badge tests "17 passed" → count réel (`cargo test` donne le nombre).
- [x] **3.4** — `docs/KNOWN_FAILURE_PATTERNS.md` (signalé manquant par `docs/METRICS.md`).
- [x] **3.5** — `cargo install cargo-deny` puis `cargo deny check` (tester `deny.toml`).
- [ ] **3.6** — 🔽 *dé-priorisé* — `docs/GOOGLE_OAUTH_SETUP.md` (cloud non prioritaire).

---

## 🧊 Icebox — volontairement reporté

| Suggestion | Pourquoi reporté |
|------------|------------------|
| **Action System** (`enum Action`) | Utile seulement pour un agent UI futur. Spéculatif tant qu'on n'a pas d'agent. |
| **Restructure complète** (`providers/`, `storage/`, `actions/`) | Gros churn pour bénéfice marginal. Structure actuelle déjà propre. On ajoute du ciblé au lieu de tout déplacer. |
| **`async_runtime.rs` global** | `std::thread::spawn` fonctionne. Pas de douleur mesurée. |
| **Semantic match (embeddings)** | Le keyword match enrichi (Phase 1) couvre l'essentiel offline. Dépendance lourde évitée. |

---

## 📌 Prochaine action

➡️ **Tâche 1.1.1** — créer `src/core/matching.rs` avec `count_skill_occurrences()`.
Suivre la boucle d'or : lire `src/core/mod.rs` d'abord → écrire → `cargo build` → cocher → commit.
