# ADR-0006 : Word-boundary skill matching + provider-scoped cache key

**Date** : 2026-06-13 | **Statut** : Accepté

## Contexte

### Problème 1 — Faux positifs dans le comptage de skills
`count_skill_occurrences()` utilisait `haystack.matches(skill).count()`, une correspondance par sous-chaîne. "rust" correspondait à "frustrated" (3 lettres communes), gonflant artificiellement le score ATS et marquant à tort un skill comme `Present` ou `Weak` alors qu'il est absent de l'offre.

### Problème 2 — Collision de cache cross-provider
`Cache::key()` ne distinguait pas les providers. Un résultat mis en cache par Ollama pouvait être réutilisé lors d'un appel à OpenAI sur le même CV+offre, produisant des sorties avec le mauvais modèle sans que l'utilisateur le sache.

## Décisions

### 1. Regex word-boundary dans `matching.rs`

Remplacer `haystack.matches(skill).count()` par :
```rust
let pattern = format!(r"\b{}\b", regex::escape(&skill));
Regex::new(&pattern).map(|re| re.find_iter(&haystack).count()).unwrap_or(0)
```

- `regex::escape()` gère les skills avec métacaractères regex (C++, C#, Node.js)
- `\b` empêche "rust" de matcher dans "frustrated", "industrial", etc.
- La crate `regex` était déjà dans `Cargo.toml` — aucune dépendance ajoutée
- Limitation connue : pour les skills terminant par des caractères `\W` (C++), le `\b` final ne s'applique pas. En pratique, ces skills sont rares et ne créent pas de faux positifs significatifs.

### 2. Provider inclus dans la clé de cache (`cache.rs`)

Signature modifiée : `key(namespace, paths, body, provider: &str)`

Le nom du provider est haché dans le SHA-256 avant les autres données. Cela garantit que `extract_cv_web-<hash>-ollama.json` et `extract_cv_web-<hash>-openai.json` sont des entrées distinctes.

Infrastructure : `LlmProviderKind::as_str()` ajouté, `LlmRouter` expose `provider_name() -> &str`, les 6 call sites dans `pipeline.rs` passent `self.llm.provider_name()`.

## Alternatives rejetées

- **Tokenization NLP** pour le word-boundary : dépendance lourde (100+ Mo), latence, offline impossible. Rejeté.
- **Prefix dans le namespace du cache** (ex. `"ollama:extract_cv_web"`) : fonctionnel mais moins propre qu'un paramètre explicite. Rejeté pour la lisibilité.
- **Clé de cache par hash de l'URL du provider** : fragile si l'URL change sans changer de provider. Rejeté.

## Conséquences

- Les scores ATS précédents (calculés avec substr-match) peuvent légèrement différer après mise à jour.
- Le cache existant est invalidé pour tous les providers (les nouvelles clés incluent le nom du provider là où les anciennes ne l'avaient pas).
- Tests ajoutés : `count_skill_occurrences_respects_word_boundary`.
