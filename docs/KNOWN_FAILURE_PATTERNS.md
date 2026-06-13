# Known Failure Patterns — HireLens

> Catalogue des modes de défaillance connus, leurs causes et leurs remèdes.
> Mettre à jour chaque fois qu'un nouveau pattern est identifié en production ou en test.
>
> Dernière mise à jour : **2026-06-13**

---

## 1. Validation — Hallucination détectée (`src/core/validation.rs`)

### 1.1 Skill inconnu proposé par le LLM

**Symptôme** : `Error: LLM attempted to introduce unsupported skill: <skill>`

**Cause** : Le LLM a inventé un skill absent de `cv.skills`. Peut arriver si le prompt
système n'est pas assez contraignant ou si le modèle ignore la liste `allowed_skills`.

**Remède** :
- Vérifier que `AdaptationRequest.allowed_skills` est bien rempli depuis `cv.skills` normalisés.
- En mode debug, logger le JSON brut retourné par le LLM avant désérialisation.
- Ce comportement est **voulu** — c'est la frontière anti-hallucination. Ne jamais contourner.

### 1.2 Bullet inventé (non traceable au CV original)

**Symptôme** : `Error: adaptation referenced a bullet not present in the original CV: <bullet>`

**Cause** : Le LLM a paraphrasé ou modifié un bullet au lieu de le copier verbatim.

**Remède** :
- Le prompt LLM doit exiger une copie exacte des bullets (`selected_bullets` = verbatim).
- Vérifier que `experience_id` correspond bien à un `Experience.id` existant dans le CV parsé.

---

## 2. LLM Providers — Erreurs de connexion

### 2.1 Ollama non démarré

**Symptôme** : `Connection refused` sur `http://localhost:11434`

**Cause** : Le processus Ollama n'est pas lancé.

**Remède** :
```bash
ollama serve          # démarrer Ollama
ollama pull llama3.1  # télécharger un modèle si absent
```

Le `FallbackProvider` (`src/llm/router.rs`) bascule automatiquement vers LM Studio si Ollama est indisponible.

### 2.2 LM Studio non démarré

**Symptôme** : `Connection refused` sur `http://localhost:1234`

**Cause** : Le serveur LM Studio n'est pas activé.

**Remède** : Dans LM Studio, activer "Local Server" dans l'onglet Developer.

### 2.3 OpenAI — clé API manquante ou invalide

**Symptôme** : `Error: OPENAI_API_KEY not set` ou `401 Unauthorized`

**Cause** : Variable d'environnement absente ou clé expirée.

**Remède** :
```bash
export OPENAI_API_KEY="sk-..."
# ou via hirelens.toml : provider = "openai"
```

### 2.4 Gemini — token OAuth2 expiré (GUI uniquement)

**Symptôme** : `401 Unauthorized` ou `invalid_grant` lors d'un appel Gemini.

**Cause** : Le refresh token a expiré (session OAuth2 périmée). Gemini est GUI-only — voir [ADR-0004](adr/0004-gemini-oauth2-gui-only.md).

**Remède** : Dans le panneau Settings de la GUI, cliquer "Disconnect" puis ré-authentifier via le flow OAuth2 PKCE.

---

## 3. Parsing CV — Erreurs YAML/Markdown

### 3.1 Frontmatter YAML malformé

**Symptôme** : `Error: failed to parse CV YAML frontmatter` ou `missing field 'skills'`

**Cause** : Le fichier Markdown ne commence pas par `---`, ou le YAML contient des erreurs
(indentation incorrecte, tabulations au lieu d'espaces, caractères spéciaux non échappés).

**Remède** :
- Valider le fichier avec `python -c "import yaml; yaml.safe_load(open('cv.md').read())"`.
- S'assurer que le frontmatter utilise des espaces (pas de tabulations).
- Les champs obligatoires : `skills`, `experience` (avec `id` + `bullets` pour chaque entrée).

### 3.2 `experience_id` manquant dans le CV

**Symptôme** : Le `selected_bullet.experience_id` ne correspond à aucune entrée → bullet rejeté.

**Cause** : Le CV ne définit pas de champ `id:` sur les entrées `experience`.

**Remède** : Ajouter un `id` unique à chaque bloc `experience` du frontmatter YAML.

---

## 4. Export PDF — Typst

### 4.1 Typst non installé

**Symptôme** : `Error: typst command not found` ou similaire.

**Cause** : Le binaire `typst` n'est pas dans le PATH.

**Remède** :
```bash
# Linux / macOS (cargo)
cargo install typst-cli

# Windows (winget)
winget install --id Typst.Typst
```

### 4.2 Fichier Markdown contenant des caractères Typst spéciaux

**Symptôme** : Rendu PDF incorrect ou erreur Typst lors de la compilation.

**Cause** : Les caractères `#`, `@`, `<`, `>` ont une signification syntaxique en Typst.

**Remède** : Le `TypstRenderer` (`src/export/typst_render.rs`) doit échapper ces caractères
avant de passer le contenu au moteur. Vérifier la fonction d'échappement si le problème persiste.

---

## 5. Cache — Corruption ou invalidation

### 5.1 Réponse LLM mise en cache mais modèle changé

**Symptôme** : Résultats incohérents entre deux runs avec le même fichier mais un modèle différent.

**Cause** : La clé de cache SHA-256 inclut le texte d'entrée mais pas le nom du modèle.

**Remède** :
```bash
rm -rf .cache/   # vider le cache manuellement
```

### 5.2 Cache corrompu (fichier JSON invalide)

**Symptôme** : `Error: failed to deserialize cached response`

**Cause** : Écriture interrompue (crash pendant l'écriture du cache).

**Remède** :
```bash
rm -rf .cache/
```

---

## 6. Modes de défaillance silencieux à surveiller

| Pattern | Risque | Détection |
|---------|--------|-----------|
| `normalize_skill()` retourne `""` sur un skill vide | Le skill est ignoré silencieusement | `SkillStatus::Weak` avec `occurrences=0` dans le rapport |
| `FallbackProvider` épuise tous les providers | Erreur confuse "no local providers available" | Vérifier que les deux providers locaux sont démarrés |
| Champ `explanations: vec![]` dans un `AuditReport` construit manuellement | Bloc "Why?" vide dans CLI/GUI/Web | Toujours remplir depuis `count_skill_occurrences()` dans `compute_audit()` |
| `serde_json::deny_unknown_fields` sur `AdaptationResponse` | Désérialisation muette si le LLM ajoute un champ | Le JSON brut du LLM doit correspondre exactement à la struct |

---

## Ajouter un pattern

Quand un nouveau mode de défaillance est identifié :
1. Ajouter une section numérotée ici avec **Symptôme / Cause / Remède**.
2. Si la cause est une décision architecturale, référencer l'ADR correspondant.
3. Mettre à jour la date "Dernière mise à jour" en en-tête.
