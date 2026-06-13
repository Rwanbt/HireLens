# ADR-0002 : `trait LlmProvider` + router multi-provider

**Date** : 2026-06-13 | **Statut** : Accepté

## Contexte

HireLens doit supporter plusieurs backends LLM (cloud, local, gratuit, payant) sans que le reste
du code ne dépende d'un provider spécifique. Les utilisateurs doivent pouvoir changer de provider
avec un seul flag CLI ou sélection GUI.

## Décision

Tout provider LLM implémente `trait LlmProvider` dans `src/llm/provider.rs` :

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn extract_skills(request: ExtractSkillsRequest) -> Result<ExtractSkillsResponse>;
    async fn generate_adaptation(request: AdaptationRequest) -> Result<AdaptationResponse>;
}
```

`LlmRouter` sélectionne l'implémentation à la création et expose la même interface. Il a deux
constructeurs :
- `LlmRouter::new(kind)` — CLI, lit config depuis env/fichier
- `LlmRouter::from_gui(kind, opts)` — GUI, utilise les settings du panneau + keyring

## Alternatives rejetées

- **Enum + match partout** : couplage fort, chaque nouvelle fonctionnalité nécessite de modifier
  tous les sites d'appel
- **Trait object par command** : trop de boilerplate, pas de gain

## Conséquences

- ✅ Ajouter un provider = 1 fichier + 1 variant enum + 3 lignes dans router
- ✅ Tests du pipeline peuvent mock `LlmProvider`
- ⚠️ `async_trait` ajoute une indirection par box — acceptable pour du I/O réseau
