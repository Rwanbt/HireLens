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

➡️ **Phase 9 + tâche 3.6 terminées** (2026-06-13) — documentation complète, 39 tests verts, clippy + fmt propres. Toutes les tâches documentées livrées. Icebox intentionnellement reporté.
Prochaine action : définir une fonctionnalité produit ou clore le sprint.

---

## 🔥 Phase 4 — Bugs critiques + sécurité (Sprint 1 — bloquants)

> Issus de la **code review complète du 2026-06-13** (voir `docs/review-2026-06-13.md`).
> **Ordre obligatoire : C1 → C2 → B1 → B2 → S1 → S4.**

### 4.1 — Anti-hallucination : failles d'intégrité

- [x] **C1 — `pipeline.rs` : snapshot `allowed_skills` avant `enrich_skills()`**
  - 📁 Fichier : `src/core/pipeline.rs`
  - 🔧 Action : capturer `allowed_skills` depuis `cv.skills` AVANT d'appeler `enrich_skills()`. Actuellement le LLM peut enrichir la whitelist et contourner `validate_adaptation()`.
  - ⚠️ C'est la faille la plus grave du projet — elle compromet l'invariant fondamental anti-hallucination.
  - ✅ Vérifier : `cargo test` — en particulier les tests `validate_adaptation`.

- [x] **C2 — `validation.rs:32` : rejeter skill vide/whitespace**
  - 📁 Fichier : `src/core/validation.rs`
  - 🔧 Action : ajouter en début de boucle :
    ```rust
    if skill.is_empty() {
        bail!("LLM returned an empty or whitespace-only skill");
    }
    ```
  - ✅ Vérifier : `cargo test`.

### 4.2 — Crash reproductible : export PDF sur CVs français

- [x] **B1 — `typst_render.rs:128` : panic UTF-8 dans `inline_markup`** *(déjà UTF-8-safe — scan d'octets sur `*` (0x2A) qui ne coïncide jamais avec un octet de continuation UTF-8 ; test de régression ajouté)*
  - 📁 Fichier : `src/export/typst_render.rs`, fonction `inline_markup`
  - 🔧 Action : réécrire en utilisant `char_indices()` au lieu d'indices byte. `"**Développeur**"` contient `é` (2 bytes) → panic garanti avec le code actuel.
  - ✅ Vérifier : `cargo test` + test manuel avec un CV contenant du `**gras**` et de l'`_italique_`.

- [x] **B2 — `typst_render.rs:121` : `escape_typst` incomplète** *(refacto DRY : source unique `push_escaped` partagée, ajoute `\ " _ [ ]`)*
  - 📁 Fichier : `src/export/typst_render.rs`, fonction `escape_typst`
  - 🔧 Action : ajouter `\`, `"`, `_`, `[`, `]` à la liste des caractères échappés.
  - ✅ Vérifier : `cargo test`.

### 4.3 — Sécurité OAuth2

- [x] **S1+S2 — `token_store.rs` : tokens OAuth2 Gemini en JSON clair sur disque** *(migré vers keyring OS, comme la clé OpenAI ; aligne le code sur ARCHITECTURE.md)*
  - 📁 Fichier : `src/auth/token_store.rs`
  - 🔧 Action : migrer vers `keyring` (déjà utilisé pour OpenAI dans `src/gui/settings.rs`). Remplacer `std::fs::write(path, json)` par `keyring::Entry::new("hirelens", "gemini-oauth-tokens")?.set_password(&json)?`.
  - ✅ Vérifier : `cargo build` + test manuel "Connexion Google" → déconnexion → reconnexion.

- [x] **S4 — `oauth_server.rs:83` : `percent_decode` absent** *(déjà implémenté — `percent_decode()` maison présent et utilisé dans `parse_query()`. Crate `percent-encoding` non ajoutée car redondante.)*

---

## 🧪 Phase 5 — Tests manquants + sécurité secondaire (Sprint 2)

### 5.1 — Tests des chemins critiques

- [x] **T1 — Tests `FallbackProvider`**
  - 📁 Fichier : `src/llm/router.rs` (module `#[cfg(test)]`)
  - 🔧 Action : tester (a) fallback déclenché sur `is_connection_error`, (b) fallback *non* déclenché sur `401 Unauthorized`, (c) épuisement des deux providers → erreur claire.
  - ✅ Vérifier : `cargo test fallback`.

- [x] **T2 — Tests `validate_adaptation` cas limites**
  - 📁 Fichier : `src/core/validation.rs`
  - 🔧 Action : tester skill vide `""`, `experience_id` inexistant, paraphrase d'un bullet (doit rejeter).
  - ✅ Vérifier : `cargo test validate`.

- [x] **T3 — Tests `compute_audit` avec `explanations`**
  - 📁 Fichier : `src/core/ats.rs`
  - 🔧 Action : vérifier que `Missing` / `Weak` / `Present` sont générés correctement selon les cas.
  - ✅ Vérifier : `cargo test compute_audit`.

- [x] **T4 — Tests `format_audit_report` avec section `Why:`**
  - 📁 Fichier : `src/cli/mod.rs`
  - 🔧 Action : tester que la section `Why:` apparaît avec les bons labels (missing/weak/present).
  - ✅ Vérifier : `cargo test format_audit`.

### 5.2 — Sécurité secondaire

- [x] **S3 — `token_store.rs:43` : TOCTOU permissions fichier** *(obsolète : la migration keyring S1+S2 supprime tout fichier sur disque)*
  - 📁 Fichier : `src/auth/token_store.rs`
  - 🔧 Action : (Unix uniquement) utiliser `OpenOptions::create_new().mode(0o600)` avant l'écriture. Note : si S1+S2 est fait (migration keyring), cette tâche devient obsolète — la cocher dans les deux cas.
  - ✅ Vérifier : `cargo build`.

- [x] **S5 — `oauth_server.rs:28` : valider le path `/callback`** *(boucle robuste : 404 aux requêtes parasites, attend le vrai callback)*
  - 📁 Fichier : `src/auth/oauth_server.rs`
  - 🔧 Action : rejeter les requêtes dont le path ne commence pas par `/callback` avec un `404`.
  - ✅ Vérifier : `cargo build`.

- [x] **M1 — `web/mod.rs:24` : bind `127.0.0.1` par défaut**
  - 📁 Fichier : `src/web/mod.rs`
  - 🔧 Action : remplacer `0.0.0.0` par `127.0.0.1`. Ajouter `--host` flag optionnel si besoin.
  - ✅ Vérifier : `cargo run -- serve` → vérifier que `http://127.0.0.1:8080` fonctionne.

- [x] **M2 — `web/mod.rs:122` : masquer les erreurs internes dans l'API**
  - 📁 Fichier : `src/web/mod.rs`, fonction `friendly_error`
  - 🔧 Action : logger `tracing::error!("{:?}", e)`, retourner un message générique côté client.
  - ✅ Vérifier : `cargo build`.

---

## 🖥️ Phase 6 — GUI egui : polish & UX (Sprint 3)

> Lire les fichiers avant d'éditer. **Ordre libre** — tâches indépendantes.

- [x] **6.1 — Avertissement "Remplissez les deux champs" : afficher uniquement après tentative**
  - 📁 Fichier : `src/gui/views/main_view.rs`, `render_controls()`
  - 🔧 Action : ajouter un booléen `app.tried_without_input: bool` mis à `true` au clic. N'afficher le warning que si `tried_without_input && !has_input`.
  - ✅ Vérifier : `cargo run -- gui` → au démarrage, aucun warning. Clic sur "Analyser" avec champs vides → warning apparaît.

- [x] **6.2 — "Pourquoi ce score ?" : ouvrir automatiquement à la première analyse**
  - 📁 Fichier : `src/gui/views/main_view.rs`, `render_audit_panel()`
  - 🔧 Action : passer `default_open(true)` sur le `CollapsingHeader` "Pourquoi ce score ?" quand `!report.explanations.is_empty()`.
  - ✅ Vérifier : `cargo run -- gui` → analyser → le bloc s'ouvre seul.

- [x] **6.3 — Provider "Gemini" : désactiver si non configuré**
  - 📁 Fichier : `src/gui/views/main_view.rs`, `render_controls()`
  - 🔧 Action : griser l'option Gemini dans le ComboBox si `app.settings.gemini.client_id.is_empty()`. Ajouter `.on_disabled_hover_text("Configurez Gemini dans ⚙️ Paramètres")`.
  - ✅ Vérifier : `cargo run -- gui`.

- [x] **6.4 — Bouton "Réinitialiser" pour repartir d'une analyse propre**
  - 📁 Fichier : `src/gui/views/main_view.rs`, `render_controls()`
  - 🔧 Action : ajouter un bouton "🔄 Réinitialiser" qui efface `cv_text`, `job_text`, `audit_state → Idle`, `adapt_state → Idle`.
  - ✅ Vérifier : `cargo run -- gui`.

- [x] **6.5 — CV optimisé : améliorer la lisibilité du panneau de résultat**
  - 📁 Fichier : `src/gui/views/main_view.rs`, `render_adapted_panel()`
  - 🔧 Action : augmenter `max_height` de `400.0` à `f32::INFINITY` (ou la hauteur disponible). Ajouter un titre de section avec le score ATS du CV adapté en évidence.
  - ✅ Vérifier : `cargo run -- gui` → optimiser un CV → vérifier que le résultat est lisible sans scroller.

- [x] **6.6 — Toolbar export : grouper visuellement les boutons**
  - 📁 Fichier : `src/gui/views/main_view.rs`, `render_adapted_panel()`
  - 🔧 Action : séparer "exporter en fichier" (💾 .md / 🌐 HTML / 📄 PDF) et "copier" (📋) avec un `ui.separator()` ou un espacement clair.
  - ✅ Vérifier : `cargo run -- gui`.

- [x] **6.7 — Settings : ouvrir seulement la section du provider actif par défaut**
  - 📁 Fichier : `src/gui/views/settings_view.rs`
  - 🔧 Action : passer `default_open(app.provider == Provider::OpenAi)` sur la section OpenAI, idem pour les autres sections.
  - ✅ Vérifier : `cargo run -- gui` → ouvrir Settings avec Ollama sélectionné → seule la section Ollama est ouverte.

- [x] **6.8 — Feedback "Export réussi" stable**
  - 📁 Fichier : `src/gui/views/main_view.rs` + `src/gui/app.rs`
  - 🔧 Action : stocker `export_feedback: Option<(String, Instant)>` dans `HireLensApp`. Auto-clear après 4s via timer dans `update()`. Utilisé pour copy + save_rx + pdf_rx.
  - ✅ Vérifier : `cargo run -- gui` → exporter en PDF → "✅ Exporté vers cv.pdf" visible 4s.

---

## 🌐 Phase 7 — UI Web : refonte & UX (Sprint 4)

> Lire `src/web/ui.html` avant d'éditer. **Ordre libre** — tâches indépendantes.

- [x] **7.1 — Remplacer `alert()` par des messages d'erreur inline**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : `<div id="error-bar">` rouge + `showError()` / `clearError()`. `readInputs()`, `analyze()`, `optimize()` n'utilisent plus `alert()`.

- [x] **7.2 — Reformater et nommer les classes CSS de façon lisible**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : CSS reformaté (une propriété par ligne). `.cg→.chip-green`, `.cr→.chip-red`, `.b-blue→.badge-blue`, `.b-green→.badge-green`, `.btn-a→.btn-primary`, `.btn-o→.btn-success`, `.btn-c→.btn-secondary`.

- [x] **7.3 — Persister CV et offre avec `localStorage`**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : `input` events → `localStorage.setItem`. IIFE `restoreFromStorage()` au chargement.

- [x] **7.4 — Ajouter le téléchargement du CV optimisé en `.md`**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : bouton "⬇️ Télécharger .md" (id `bdl`) via `Blob` + `URL.createObjectURL` + `a.download = 'cv-optimized.md'`.

- [x] **7.5 — Empty state : afficher un placeholder avant la première analyse**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : `<div id="empty-state">` visible au démarrage ; caché dans `renderAudit()` via `.classList.add('hidden')`.

- [x] **7.6 — Provider select : indiquer Gemini comme GUI-only**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : `<option disabled>🌟 Gemini (GUI uniquement)</option>` ajouté dans le select.

- [x] **7.7 — Textareas : passer à `min-height` responsive**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : `height:240px` → `min-height:160px; max-height:50vh`.

- [x] **7.8 — Structurer le JS en sections commentées**
  - 📁 Fichier : `src/web/ui.html`
  - 🔧 Action : `<script>` découpé en `// === Utilitaires ===`, `// === Rendu ===`, `// === API ===`, `// === Événements ===`. `inputs()` renommé `readInputs()`, `color()→scoreColor()`, `none()→noneLabel()`.

---

## 🔧 Phase 8 — Robustesse + documentation (Sprint 5)

> **Ordre libre** — tâches indépendantes.

- [x] **8.1 — `matching.rs:35` : word-boundary matching**
  - 📁 Fichier : `src/core/matching.rs`
  - 🔧 Action : remplacer `haystack.matches(skill.as_str()).count()` par une regex word-boundary `\bskill\b`. `regex` déjà dans Cargo.toml.
  - ⚠️ Ce changement modifie les scores — vérifier que les tests de scoring restent cohérents.
  - ✅ Vérifier : `cargo test`.

- [x] **8.2 — `router.rs:106` : utiliser `reqwest::Error::is_connect()`**
  - 📁 Fichier : `src/llm/router.rs`, fonction `is_connection_error`
  - 🔧 Action : `downcast_ref::<reqwest::Error>()` → `is_connect()` en priorité ; fallback string matching pour les erreurs non-reqwest (mocks dans les tests).
  - ✅ Vérifier : `cargo build`.

- [x] **8.3 — `oauth_server.rs` : vérifier le paramètre `state` PKCE**
  - 📁 Fichier : `src/auth/google.rs`
  - 🔧 Action : déjà implémenté — `google.rs:48` vérifie `if returned_state != state { bail!(...) }`. *(Confirmation au 2026-06-13)*
  - ✅ Vérifier : `cargo build`.

- [x] **8.4 — `router.rs` : URLs FallbackProvider lues depuis `Config`**
  - 📁 Fichier : `src/llm/router.rs`
  - 🔧 Action : `new_local_with_fallback()` appelle `Config::load()` → `config.ollama_base_url()` / `config.lmstudio_base_url()` avec env vars (`OLLAMA_BASE_URL`, `LMSTUDIO_BASE_URL`) et fichier `hirelens.toml`.
  - ✅ Vérifier : `cargo test`.

- [x] **8.5 — `cache.rs` : inclure le provider dans la clé de cache**
  - 📁 Fichier : `src/utils/cache.rs` + `src/core/pipeline.rs`
  - 🔧 Action : `key()` prend un paramètre `provider: &str` inclus dans le hash SHA-256. `LlmRouter` expose `provider_name()` ; `Pipeline` le passe aux 6 call sites.
  - ✅ Vérifier : `cargo test`.

- [x] **3.6** — `docs/GOOGLE_OAUTH_SETUP.md` : guide pas-à-pas complet (7 étapes + troubleshooting + diagramme flow).

---

## 📄 Phase 9 — Documentation & polish (Sprint 6)

> **Ordre libre** — tâches indépendantes.

- [x] **9.1 — `.gitattributes` : normaliser les fins de ligne**
  - 🔧 Action : `* text=auto eol=lf` + règles par extension. Fin des warnings CRLF à chaque commit.
  - ✅ Vérifier : `git commit` sans warnings.

- [x] **9.2 — `CONTRIBUTING.md` : guide de contribution**
  - 🔧 Action : invariant anti-hallucination, protocole d'ajout de provider (4 étapes), conventions code, format commit, limite PR 400 LOC, glossaire domaine.
  - ✅ Vérifier : `cat CONTRIBUTING.md` → complet.

- [x] **9.3 — `CHANGELOG.md` : traçabilité des releases**
  - 🔧 Action : reconstruit depuis les phases (dev → alpha → beta → unreleased).
  - ✅ Vérifier : `cat CHANGELOG.md` → cohérent avec `git log`.

- [x] **9.4 — `hirelens.example.toml` : config annotée**
  - 🔧 Action : enrichi avec env vars par section, note Gemini GUI-only, avertissement OPENAI_API_KEY.
  - ✅ Vérifier : `toml::from_str(content)` → parse sans erreur (couvert par le parser de Config).

- [x] **9.5 — `docs/adr/0006-word-boundary-and-provider-cache-key.md`**
  - 🔧 Action : documente les deux décisions Phase 8 (regex `\b`, clé cache + provider).
  - ✅ Vérifier : fichier lisible, alternatives documentées.

---
