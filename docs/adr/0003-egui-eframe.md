# ADR-0003 : egui/eframe pour la GUI

**Date** : 2026-06-13 | **Statut** : Accepté

## Contexte

HireLens a démarré comme un outil CLI pur. Une GUI a été ajoutée pour rendre l'outil accessible
sans terminal. Il faut choisir un toolkit Rust natif.

## Décision

**eframe 0.29 + egui 0.29** (immediate-mode GUI, backend wgpu).

Raisons :
- Même version que Seno DAW (capital de patterns réutilisable, connaissance existante)
- Bundle single-binary sans dépendance système (wgpu backend)
- Suffisant pour les besoins : formulaire + résultats + settings panel
- Compile sur Windows, Linux, macOS sans configuration par plateforme

## Alternatives rejetées

- **Tauri** : overhead JS/HTML, complexité build, overkill pour un outil utilitaire
- **iced** : manque de widgets (AccessKit absent en 0.12), API moins mature qu'egui
- **slint** : licence commerciale pour usage non-GPL

## Conséquences

- ✅ Cohérence avec Seno DAW — patterns partagés (COL_*, NativeOptions)
- ✅ Single binary, pas de runtime JS/HTML
- ⚠️ Immediate-mode = logique métier et rendu découplés — ne jamais mettre de logique dans `update()`
- ⚠️ Pas de rendu HTML natif (HTML export = fichier séparé via `gui/html_export.rs`)
