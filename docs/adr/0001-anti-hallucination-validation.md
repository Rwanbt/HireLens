# ADR-0001 : Validation anti-hallucination post-LLM côté Rust

**Date** : 2026-06-13 | **Statut** : Accepté

## Contexte

HireLens utilise des LLMs pour extraire les compétences d'une offre d'emploi et proposer une
adaptation du CV. Les LLMs sont susceptibles d'inventer des compétences ou des bullets non présents
dans le CV original (*hallucination*). Dans un outil de CV professionnel, tout contenu inventé
détruirait la confiance et pourrait causer du tort à l'utilisateur.

## Décision

Toute proposition LLM passe obligatoirement par `core::validation::validate_adaptation()` avant
d'atteindre le renderer. Cette fonction vérifie :

1. **Skill whitelist** : chaque compétence proposée doit exister dans `cv.skills` après normalisation
2. **Bullet traceback** : chaque bullet adapté doit exister verbatim dans `cv.experience[*].bullets`

Les LLMs ne retournent que du **JSON structuré** (`AdaptationResponse`) — jamais de texte libre
qui serait rendu directement. La validation est effectuée par Rust avant le renderer.

## Alternatives rejetées

- **Guardrails LLM (prompt engineering)** : non fiable, contournable, non vérifiable
- **Validation côté LLM** (demander au modèle de vérifier lui-même) : circulaire, inefficace
- **Trust the LLM** : inacceptable pour un outil professionnel

## Conséquences

- ✅ La promesse produit est garantie par code, pas par des heuristiques
- ✅ Les tests de `validate_adaptation()` prouvent le comportement
- ⚠️ Limite l'adaptation à la reformulation des bullets existants (pas d'invention)
- ⚠️ Un CV mal structuré (bullets trop vagues) produit une adaptation peu différenciante
