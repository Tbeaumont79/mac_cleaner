//! Zones du dossier personnel où l'utilisateur ne doit pas supprimer de
//! fichiers à la main : données internes des apps et bibliothèques
//! (Photos, Musique…) dont les fichiers n'ont de sens que pour leur app.

use std::path::Path;

const BUNDLE_EXTENSIONS: &[&str] = &[
    "app",
    "photoslibrary",
    "photolibrary",
    "musiclibrary",
    "tvlibrary",
    "imovielibrary",
    "fcpbundle",
    "logicx",
    "band",
    "aplibrary",
];

pub fn is_protected_dir(home: &Path, dir: &Path) -> bool {
    // APFS est insensible à la casse par défaut : ~/library désigne ~/Library.
    if dir.parent() == Some(home) {
        if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
            if name.eq_ignore_ascii_case("Library") || name.eq_ignore_ascii_case(".Trash") {
                return true;
            }
        }
    }
    dir.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| BUNDLE_EXTENSIONS.iter().any(|b| b.eq_ignore_ascii_case(e)))
}

pub fn within_protected(home: &Path, path: &Path) -> bool {
    path.ancestors()
        .take_while(|a| *a != home)
        .any(|a| is_protected_dir(home, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_and_bundles_are_protected() {
        let home = Path::new("/Users/u");
        assert!(within_protected(home, Path::new("/Users/u/Library/Containers/x/big.raw")));
        assert!(within_protected(home, Path::new("/Users/u/library/x")));
        assert!(within_protected(home, Path::new("/Users/u/.Trash/a.iso")));
        assert!(within_protected(
            home,
            Path::new("/Users/u/Pictures/Photos Library.photoslibrary/originals/a.heic")
        ));
        assert!(within_protected(home, Path::new("/Users/u/Applications/X.APP/Contents/big")));
    }

    #[test]
    fn regular_files_are_allowed() {
        let home = Path::new("/Users/u");
        assert!(!within_protected(home, Path::new("/Users/u/Downloads/ubuntu.iso")));
        assert!(!within_protected(home, Path::new("/Users/u/Documents/Library/notes.pdf")));
        assert!(!within_protected(home, Path::new("/Users/u/Movies/app.mov")));
    }
}
