//! Liste blanche des dossiers nettoyables.
//!
//! C'est le cœur du modèle de sécurité de l'application :
//! - chaque catégorie pointe vers des chemins **statiques** (ou un motif borné,
//!   cf. `PathSpec::ContainerEach`), jamais calculés depuis une entrée
//!   utilisateur ;
//! - tous les chemins résolus restent à l'intérieur du dossier personnel (`~`),
//!   jamais `/System` ni `/Library` racine ;
//! - rien ici n'est supprimé : ce module ne fait que *décrire* quoi scanner.
//!
//! Pour ajouter une catégorie, on ajoute une entrée dans `categories()`. On ne
//! permet jamais de passer un chemin arbitraire depuis le frontend.

use std::path::PathBuf;

/// Identifiant stable d'une catégorie (utilisé comme clé côté frontend aussi).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CategoryId {
    UserCaches,
    ContainerCaches,
    UserLogs,
    SavedAppState,
    XcodeDerivedData,
    Trash,
}

/// Façon de résoudre les chemins d'une catégorie, toujours sous `~`.
pub enum PathSpec {
    /// Chemin relatif au dossier personnel : `~/<rel>`.
    Home(&'static str),
    /// Sous-chemin répété dans chaque conteneur d'app sandboxée :
    /// `~/Library/Containers/<app>/<subpath>` pour chaque `<app>`.
    ContainerEach(&'static str),
}

/// Description d'une catégorie nettoyable.
pub struct Category {
    pub id: CategoryId,
    /// Nom lisible affiché dans l'UI.
    pub label: &'static str,
    /// Courte explication de ce qui est nettoyé.
    pub description: &'static str,
    /// Comment résoudre les chemins concrets de cette catégorie.
    pub paths: &'static [PathSpec],
    /// `true` => suppression **définitive** (pas la corbeille). Réservé aux cas
    /// où la corbeille n'a pas de sens, p. ex. vider la corbeille elle-même.
    pub permanent: bool,
}

/// Toutes les catégories que l'application est autorisée à scanner / nettoyer.
///
/// L'ordre détermine l'ordre d'affichage dans l'UI.
pub fn categories() -> &'static [Category] {
    &[
        Category {
            id: CategoryId::UserCaches,
            label: "Caches applications",
            description: "Caches régénérables des apps (~/Library/Caches).",
            paths: &[PathSpec::Home("Library/Caches")],
            permanent: false,
        },
        Category {
            id: CategoryId::ContainerCaches,
            label: "Caches d'apps sandboxées",
            description: "Caches des apps en bac à sable (Safari, etc.).",
            paths: &[PathSpec::ContainerEach("Data/Library/Caches")],
            permanent: false,
        },
        Category {
            id: CategoryId::UserLogs,
            label: "Logs utilisateur",
            description: "Journaux d'applications (~/Library/Logs).",
            paths: &[PathSpec::Home("Library/Logs")],
            permanent: false,
        },
        Category {
            id: CategoryId::SavedAppState,
            label: "États d'apps sauvegardés",
            description: "Fenêtres/onglets restaurés au lancement des apps.",
            paths: &[PathSpec::Home("Library/Saved Application State")],
            permanent: false,
        },
        Category {
            id: CategoryId::XcodeDerivedData,
            label: "Xcode DerivedData",
            description: "Caches de build Xcode, entièrement régénérables.",
            paths: &[PathSpec::Home("Library/Developer/Xcode/DerivedData")],
            permanent: false,
        },
        Category {
            id: CategoryId::Trash,
            label: "Corbeille",
            description: "Vide la corbeille, sans retour possible.",
            paths: &[PathSpec::Home(".Trash")],
            permanent: true,
        },
    ]
}

/// Retrouve une catégorie par son identifiant.
pub fn find(id: CategoryId) -> Option<&'static Category> {
    categories().iter().find(|c| c.id == id)
}

/// `true` si la catégorie dépend de `~/Library/Containers` et que macOS en refuse la lecture.
pub fn containers_denied(cat: &Category) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    cat.paths.iter().any(|spec| matches!(spec, PathSpec::ContainerEach(_)))
        && std::fs::read_dir(home.join("Library/Containers"))
            .is_err_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
}

/// Résout les chemins absolus d'une catégorie sous le `$HOME` de l'utilisateur.
///
/// Garde-fou : chaque chemin résolu est un vrai dossier sous `~`, sans lien
/// symbolique sur le trajet (`real_dir`).
pub fn resolved_paths(cat: &Category) -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let home = std::fs::canonicalize(&home).unwrap_or(home);

    let mut out = Vec::new();
    for spec in cat.paths {
        match spec {
            PathSpec::Home(rel) => out.extend(real_dir(home.join(rel))),
            PathSpec::ContainerEach(sub) => {
                let containers = home.join("Library/Containers");
                if let Ok(read) = std::fs::read_dir(&containers) {
                    for entry in read.filter_map(Result::ok) {
                        if entry.file_type().is_ok_and(|t| t.is_dir()) {
                            out.extend(real_dir(entry.path().join(sub)));
                        }
                    }
                }
            }
        }
    }
    out
}

/// Garde le dossier seulement s'il existe et qu'aucun maillon n'est un lien
/// symbolique : une app sandboxée pourrait remplacer son dossier de caches par
/// un lien vers ~/Documents.
fn real_dir(expected: PathBuf) -> Option<PathBuf> {
    let real = std::fs::canonicalize(&expected).ok()?;
    (real == expected && real.is_dir()).then_some(real)
}

#[cfg(test)]
mod tests {
    use super::real_dir;

    #[test]
    fn real_dir_rejects_symlinks() {
        let base = std::fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join("mac_cleaner_real_dir_test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("real/Caches")).unwrap();
        std::os::unix::fs::symlink(base.join("real"), base.join("link")).unwrap();

        let direct = real_dir(base.join("real/Caches"));
        let through_link = real_dir(base.join("link/Caches"));
        let missing = real_dir(base.join("real/Missing"));
        std::fs::remove_dir_all(&base).unwrap();

        assert!(direct.is_some());
        assert!(through_link.is_none());
        assert!(missing.is_none());
    }
}
