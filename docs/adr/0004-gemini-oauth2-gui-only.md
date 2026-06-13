# ADR-0004 : Gemini GUI-only via OAuth2 PKCE

**Date** : 2026-06-13 | **Statut** : Accepté

## Contexte

Google Gemini nécessite une authentification. Il est possible d'utiliser une clé API service
(stockée dans l'env) ou OAuth2 PKCE (flux utilisateur, token personnel). Pour les outils
professionnels publics, OAuth2 PKCE est plus approprié : pas de clé API exposée, l'utilisateur
s'authentifie avec son propre compte Google.

## Décision

Gemini est **GUI-only** via OAuth2 PKCE :

1. L'utilisateur configure `client_id` + `client_secret` dans le panneau Settings
2. Il clique "Se connecter à Google" → flux PKCE → token stocké dans le keyring OS
3. `LlmRouter::from_gui(Gemini, opts)` récupère le token valide avant chaque appel LLM
4. `LlmRouter::new(Gemini)` retourne une erreur explicite (intentionnel)

Le token est stocké dans le keyring OS (`keyring` crate) — pas dans un fichier plain-text.

## Alternatives rejetées

- **Clé API dans env var** : exposée dans l'historique shell, inconfortable pour les utilisateurs non-dev
- **Clé API dans fichier config** : risque de commit accidentel
- **Gemini disponible en CLI** : le flux OAuth2 interactif (ouvre un navigateur, attend un redirect)
  est incompatible avec un pipeline CLI non-interactif

## Conséquences

- ✅ Pas de clé API exposée dans les fichiers de config
- ✅ Token stocké de façon sécurisée par l'OS (keyring Windows Credential Manager, etc.)
- ⚠️ Gemini inutilisable dans les pipelines CI/CD (intentionnel — utiliser Ollama ou OpenAI pour ça)
- ⚠️ Nécessite que l'utilisateur crée un projet Google Cloud avec OAuth2 credentials
