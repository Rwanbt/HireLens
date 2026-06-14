# ADR-0007 : Accès Gemini — client OAuth embarqué + clé API par utilisateur

**Date** : 2026-06-14 | **Statut** : Accepté | **Met à jour** : ADR-0004

## Contexte

ADR-0004 a choisi OAuth2 PKCE GUI-only pour Gemini afin de ne pas exposer de clé
API partagée. L'implémentation initiale demandait cependant à **chaque utilisateur
final** de créer son propre client OAuth Google Cloud puis de saisir Client ID +
Client Secret dans les Paramètres — une expérience inacceptable pour un produit
grand public (« personne n'a envie de faire ça »).

## Décision

Deux chemins d'authentification Gemini, **sans saisie d'identifiants OAuth par
l'utilisateur** :

1. **« Se connecter avec Google » (principal)** — l'app embarque un client OAuth
   de type « Application de bureau » via des variables d'environnement au build
   (`HIRELENS_GOOGLE_CLIENT_ID` / `HIRELENS_GOOGLE_CLIENT_SECRET`, lues par
   `option_env!`, donc **absentes du dépôt git**). Le clic ouvre directement la
   page de connexion Google. Pour un client « Desktop app », Google considère le
   secret comme non-confidentiel et PKCE assure la vraie protection.
2. **Clé API Gemini (repli)** — l'utilisateur colle sa propre clé gratuite
   (Google AI Studio), stockée dans le **keyring OS**. Elle est envoyée en
   `Authorization: Bearer` sur l'endpoint OpenAI-compatible — exactement le même
   chemin que le token OAuth — donc **aucun code provider supplémentaire**.

Un override « Client OAuth personnalisé (avancé) » reste disponible pour qui veut
utiliser son propre projet Google Cloud.

Résolution dans `auth::google::resolve_client` : client utilisateur s'il est
renseigné, sinon client embarqué. `LlmRouter::from_gui` préfère la clé API si
présente, sinon lance le flux OAuth.

## Alternatives rejetées

- **Statu quo (chaque utilisateur saisit Client ID/Secret)** : UX rédhibitoire.
- **Clé API partagée embarquée dans l'app** : exposerait un secret commun + des
  quotas partagés — c'est précisément ce qu'ADR-0004 voulait éviter.
- **Clé API uniquement** : simple, mais perd le « login Google » explicitement
  demandé.

## Conséquences

- **+** UX conforme : un clic (OAuth embarqué) ou un collage de clé (instantané),
  plus aucun champ Client ID/Secret par défaut.
- **+** Respecte l'esprit d'ADR-0004 : pas de secret partagé donnant accès à une
  ressource commune ; le secret OAuth desktop est non-confidentiel par design
  Google ; la clé API est strictement par-utilisateur.
- **−** L'app doit enregistrer **un** client OAuth Google et passer la
  **vérification Google** pour un usage public (sinon écran « application non
  vérifiée » + allowlist de testeurs en mode Testing). Le scope
  `generativelanguage` est sensible.
- **−** Le binaire distribué doit être buildé avec les variables d'env ; sans
  elles, le bouton « Se connecter avec Google » est désactivé et seule la clé API
  fonctionne (dégradation gracieuse, message explicite dans l'UI).
