# ADR-0008 : Moteur de matching offline 100 % algorithmique (sans IA)

**Date** : 2026-06-14 | **Statut** : Accepté

## Contexte

Le mode offline ne faisait que recopier tous les bullets du CV sans tri
(`llm/http_json.rs::offline_adaptation`, un passthrough). HireLens avait besoin
d'un vrai moteur de matching CV ↔ offre **déterministe, sans réseau ni modèle**,
pour scorer et adapter un CV hors-ligne — un argument produit (gratuit, privé,
instantané, reproductible).

Décision conçue dans la RFC `offline-matching-algorithm.md` (v3, deux tours de
review 5 IA), implémentée en P1→P4 sur `feat/offline-matching`.

## Décision

Un moteur algorithmique pur, dans `core/`, qui ne fait que **sélectionner et
classer de l'existant** — jamais générer.

**Architecture (dépendance `pipeline → core`, jamais `core → llm`)** :
- `core/offline_match.rs::run(cv, job, allowed_skills) -> OfflineMatchResult` —
  struct métier propre (PAS `AdaptationResponse`, un DTO LLM). `pipeline.rs`
  bifurque tôt (`if offline { core::offline_match::run } else { provider }`) et
  mappe le résultat sur les DTOs ; c'est le **seul** pont `core ↔ llm`.
- `core/skills.rs` — dico regroupé par `SkillCategory` (source unique), table
  d'alias TOML embarquée (`include_str!` + override `HIRELENS_ALIASES_FILE`),
  skills ambigus (`go/r/c/spring/swift/dart`) comptés seulement sur casse
  significative ou contexte n-gram, négation **formes absolues uniquement**.
- `core/matching.rs` — `keyword_coverage` (tokens non-skill/non-stopword) et
  `weighted_requirements` (fréquence saturée × titre × section, cues FR+EN).
- `core/similarity.rs` — `lexical_similarity` (overlap TF + saturation BM25-lite).
- `core/ats.rs` — `MatchSignals` + `blend` : `0.45·skill_cov + 0.15·keyword_cov
  + 0.40·lexical_sim`, gaté multiplicativement par `structure_factor ∈
  {1.0,0.9,0.75}`. Bascule non-tech (`Σ poids skills == 0` → `0.80·lexical +
  0.20·keyword`, plancher 0.20 → 0). Entrée vide → 0 explicite.

**Anti-hallucination (renforcée, non négociable)** : le moteur sélectionne les
bullets **par index** (`Vec<usize>` dans `cv.experience[x].bullets`) et copie la
`String` verbatim uniquement à la frontière de sortie. `prioritized_skills` est
toujours un simple ré-ordonnancement de l'allowed-set. `core/validation.rs` reste
la frontière de sécurité et **n'est jamais touchée ni affaiblie** ; `skill_set`
reste normalize-only (jamais alias-canonicalisé) pour ne pas élargir la whitelist.

## Alternatives rejetées

- **Embeddings/NN locaux (MiniLM, candle/ort)** — c'est un *modèle*, hors du
  cadre « sans IA » ; noté « semantic offline » futur.
- **Offline comme `LlmProvider`** — couplait `core` à `llm` et fuyait le DTO LLM ;
  remplacé par une fonction pure bifurquée dans le pipeline.
- **TF-IDF classique** — dégénéré sur N=2 documents (idf=0 sur les termes communs).
- **Fuzzy matching (`strsim`)** *(différé)* — optionnel dans la RFC ; ajoute une
  dépendance et un risque de faux positifs contraire à la précision visée. À
  réévaluer si le jeu d'éval montre un déficit de rappel.
- **Poids du blend configurables (`hirelens.toml`)** *(différé)* — il n'existe pas
  encore de jeu d'éval annoté (RFC §14) pour calibrer ; exposer ces knobs
  maintenant serait de la surface de config prématurée (les scores V1 sont
  reconnus non calibrés, RFC §15). Les poids restent des constantes centralisées
  dans `ats.rs` ; le câblage config est trivial à ajouter quand le jeu d'éval
  existera.

## Conséquences

- ✅ Anti-hallucination **gratuite par construction** : sélection/classement,
  jamais de génération. Property test (égalité stricte `==`) + test canari.
- ✅ Déterministe, hors-ligne, instantané, privé.
- ✅ `core` indépendant de `llm` (pas de cycle) ; ajout d'un signal = local à `core`.
- ⚠️ Scores **heuristiques et relatifs**, non calibrés au point près entre paires
  très différentes (RFC §15). Le jeu d'éval annoté guidera le tuning.
- ⚠️ « Mention ≠ expérience » : « curieux d'apprendre Rust » pèse comme « 5 ans de
  Rust » — limite intrinsèque du sans-IA, à exposer dans l'UI.
- ⚠️ Le tokenizer simple laisse coller un `.` final (`c++.`) ; le tokenizer
  state-machine (`core/text.rs::tokenize`) gère les skills symboliques pour la
  similarité, mais l'extraction par regex garde la limite (corrigée si besoin en
  étendant la classe de frontière).

## Phases livrées

P1 (taxonomie + offre pondérée + négation + refactor) · P2 (similarité lexicale
+ blend `MatchSignals`) · P3 (classement des bullets par `relevance` + top-K) ·
P4 (golden + property tests + cette ADR). N-grams et BM25 + IDF-min différés
(escalade conditionnelle si le golden non-tech est mal noté).
