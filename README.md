# Sweep

Nettoyeur Mac sûr par conception, construit avec Tauri 2, Vue 3 et Rust.

## Fonctionnalités

- **Analyse intelligente** : un seul scan pour les déchets système et les fichiers volumineux.
- **Nettoyage** : caches, logs, états d'apps, Xcode DerivedData et corbeille.
- **Fichiers volumineux** : les plus gros fichiers du dossier personnel, triés par taille.

## Modèle de sécurité

- Le nettoyage ne touche qu'une liste blanche de dossiers sous `~` (`src-tauri/src/targets.rs`). Le frontend envoie des identifiants de catégorie, jamais de chemins.
- Tout part à la corbeille. Seul « vider la corbeille » supprime définitivement, et cette catégorie n'est jamais présélectionnée.
- Les fichiers désignés à la main sont revalidés côté Rust : ils doivent être sous `~` et hors des zones protégées (`~/Library`, bibliothèques Photos et Musique, apps), et un lien symbolique est mis à la corbeille sans toucher à sa cible (`src-tauri/src/guard.rs`).
- Certaines catégories nécessitent l'« Accès complet au disque » ; l'app le signale quand macOS refuse la lecture.

## Développement

Prérequis : Rust (`rustup`), Node et pnpm.

```sh
pnpm install
pnpm tauri dev       # lancer l'app
pnpm tauri build     # produire Sweep.app et le .dmg
cd src-tauri && cargo test
```
