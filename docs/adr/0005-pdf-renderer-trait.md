# ADR-0005 : Trait `PdfRenderer` + `TypstRenderer`

**Date** : 2026-06-13 | **Statut** : Accepté

## Contexte

`src/export/typst_render.rs` exposait une fonction libre `export_pdf()` appelée directement
depuis `gui/app.rs`. Cette dépendance concrète rendait impossible de :
- tester `start_export_pdf()` sans déclencher la compilation Typst complète
- substituer un renderer alternatif (Pandoc, WeasyPrint, mock) sans modifier l'appelant

## Décision

Introduction du trait `PdfRenderer` dans `src/export/mod.rs` :

```rust
pub trait PdfRenderer {
    fn render(&self, markdown: &str) -> anyhow::Result<Vec<u8>>;
}
```

`TypstRenderer` dans `typst_render.rs` implémente ce trait en déléguant à `export_pdf()`.
`gui/app.rs` appelle `TypstRenderer.render()` au lieu de `export_pdf()` directement.

## Alternatives rejetées

- **Garder `export_pdf()` directement** : plus simple à court terme, mais verrouille l'impl.
- **Box<dyn PdfRenderer>** stocké dans `AppState` : over-engineering — un seul renderer actif, pas besoin d'indirection dynamique.

## Conséquences

- La frontière de test est propre : un futur `MockPdfRenderer` peut simuler une erreur disque.
- `export_pdf()` reste `pub` (utilisée par les tests unitaires Typst).
- Si un 2e renderer est ajouté (Pandoc fallback), l'appelant n'a pas à changer.
