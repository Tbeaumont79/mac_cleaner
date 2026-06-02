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
            description: "Vide la corbeille. ⚠️ Suppression définitive.",
            paths: &[PathSpec::Home(".Trash")],
            permanent: true,
        },
    ]
}

/// Retrouve une catégorie par son identifiant.
pub fn find(id: CategoryId) -> Option<&'static Category> {
    categories().iter().find(|c| c.id == id)
}

/// Résout les chemins absolus d'une catégorie sous le `$HOME` de l'utilisateur.
///
/// Garde-fou : chaque chemin résolu doit rester **à l'intérieur** du dossier
/// personnel. Tout chemin qui s'en échappe (`..`, lien, etc.) est écarté.
pub fn resolved_paths(cat: &Category) -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for spec in cat.paths {
        match spec {
            PathSpec::Home(rel) => {
                let p = home.join(rel);
                if p.starts_with(&home) {
                    out.push(p);
                }
            }
            PathSpec::ContainerEach(sub) => {
                let containers = home.join("Library/Containers");
                if let Ok(read) = std::fs::read_dir(&containers) {
                    for entry in read.filter_map(Result::ok) {
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        if !is_dir {
                            continue;
                        }
                        let p = entry.path().join(sub);
                        if p.starts_with(&home) {
                            out.push(p);
                        }
                    }
                }
            }
        }
    }
    out
}
