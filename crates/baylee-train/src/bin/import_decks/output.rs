//! Where the imported deck files are written, and the path the manifest names
//! each one by.
//!
//! Readers of the manifest (`eval4`, `deck_catalog4`) resolve a deck's path
//! against the checkout they were built in, `repo_root().join(path)`. A
//! relative `--imported-dir` therefore keeps the decks inside that checkout,
//! as the default does; an absolute one keeps them outside it, where they stay
//! as imported while the checkout moves on, and the manifest names them
//! absolutely so that every build reads the same files.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;

/// The three directories of an import.
#[derive(Clone, Copy, Debug)]
pub enum DeckKind {
    /// A preconstructed deck as printed.
    Precon,
    /// A preconstructed deck with replacements for cards the engine lacks.
    Adapted,
    /// A deck the importer builds itself.
    Curated,
}

impl DeckKind {
    const ALL: [Self; 3] = [Self::Precon, Self::Adapted, Self::Curated];

    const fn dir(self) -> &'static str {
        match self {
            Self::Precon => "precon",
            Self::Adapted => "adapted",
            Self::Curated => "curated",
        }
    }
}

/// The import's deck directory.
pub struct DeckOut {
    /// Where the files go.
    dir: PathBuf,
    /// `--imported-dir` as given, without a trailing separator.
    named: String,
}

impl DeckOut {
    /// The deck directory `imported_dir`, relative to `root` unless absolute.
    pub fn new(root: &Path, imported_dir: &Path) -> anyhow::Result<Self> {
        let named = imported_dir
            .to_str()
            .context("--imported-dir must be UTF-8: the manifest names decks by it")?
            .trim_end_matches('/')
            .to_owned();
        anyhow::ensure!(!named.is_empty(), "--imported-dir is empty");
        Ok(Self {
            dir: root.join(imported_dir),
            named,
        })
    }

    /// Creates the three directories.
    pub fn create(&self) -> anyhow::Result<()> {
        for kind in DeckKind::ALL {
            let dir = self.dir.join(kind.dir());
            fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        Ok(())
    }

    /// Writes one deck file and returns the path the manifest names it by.
    pub fn write(&self, kind: DeckKind, file: &str, text: &str) -> anyhow::Result<String> {
        let path = self.dir.join(kind.dir()).join(file);
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(format!("{}/{}/{file}", self.named, kind.dir()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("import-decks-output-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_default_directory_names_decks_as_before() {
        let root = scratch("default");
        let out = DeckOut::new(&root, Path::new("data/decks/imported")).unwrap();
        out.create().unwrap();
        let named = out
            .write(DeckKind::Adapted, "a-adapted.txt", "deck")
            .unwrap();
        assert_eq!(named, "data/decks/imported/adapted/a-adapted.txt");
        assert_eq!(fs::read_to_string(root.join(&named)).unwrap(), "deck");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_absolute_directory_receives_the_decks_and_leaves_the_checkout_alone() {
        let root = scratch("checkout");
        let elsewhere = scratch("elsewhere");
        let out = DeckOut::new(&root, &elsewhere.join("imported/")).unwrap();
        out.create().unwrap();
        let named = out.write(DeckKind::Precon, "p.txt", "deck").unwrap();
        assert_eq!(
            Path::new(&named),
            elsewhere.join("imported/precon/p.txt").as_path()
        );
        // A reader resolves the manifest's path against its checkout, as
        // `eval4` does, and finds the file where it was written.
        assert_eq!(fs::read_to_string(root.join(&named)).unwrap(), "deck");
        assert_eq!(
            fs::read_dir(&root).unwrap().count(),
            0,
            "the checkout gained files"
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(elsewhere).unwrap();
    }
}
