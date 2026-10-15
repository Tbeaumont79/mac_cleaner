# Sweep

Nettoyeur Mac sûr par conception, construit avec Tauri 2, Vue 3 et Rust.

## Fonctionnalités

- **Analyse intelligente** : un seul scan pour le nettoyage, la protection et les performances, puis tout lancer en un clic.
- **Nettoyage** : caches, logs, états d'apps, Xcode DerivedData et corbeille.
- **Protection** : vérifie que SIP, Gatekeeper, FileVault, le coupe-feu et les mises à jour de sécurité sont actifs, et audite la signature des éléments lancés automatiquement. Sweep n'est pas un antivirus.
- **Performance** : tâches de maintenance (DNS, Spotlight, LaunchServices, Quick Look, polices, mémoire) et activation ou désactivation réversible des agents de démarrage.
- **Applications** : désinstalle une app avec ses fichiers associés dans `~/Library`.
- **Fichiers volumineux** : les plus gros fichiers du dossier personnel, triés par taille.

## Modèle de sécurité

- Le nettoyage ne touche qu'une liste blanche de dossiers sous `~` (`src-tauri/src/targets.rs`). Le frontend envoie des identifiants de catégorie, jamais de chemins.
- Tout part à la corbeille. Seul « vider la corbeille » supprime définitivement, et cette catégorie n'est jamais présélectionnée.
- Les fichiers désignés à la main sont revalidés côté Rust : ils doivent être sous `~` et hors des zones protégées (`~/Library`, bibliothèques Photos et Musique, apps), et un lien symbolique est mis à la corbeille sans toucher à sa cible (`src-tauri/src/guard.rs`).
- Le désinstallateur recalcule côté Rust les fichiers associés à partir d'un identifiant de bundle strictement validé ; il refuse les apps Apple, les apps ouvertes et Sweep lui-même.
- Les tâches de maintenance sont une liste blanche de commandes fixes ; celles qui demandent des droits administrateur passent par une seule demande de mot de passe macOS.
- Certaines catégories nécessitent l'« Accès complet au disque » ; l'app le signale quand macOS refuse la lecture.

## Développement

Prérequis : Rust (`rustup`), Node et pnpm.

```sh
pnpm install
pnpm tauri dev       # lancer l'app
pnpm tauri build     # produire Sweep.app et le .dmg
cd src-tauri && cargo test
```
