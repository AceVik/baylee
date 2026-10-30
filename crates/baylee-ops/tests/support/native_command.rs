//! Locate Cargo's non-shipped native command fixture, as the seat CLI tests do.
use std::path::{Path, PathBuf};

pub fn install(directory: &Path, names: &[&str]) {
    let example = executable();
    for name in names {
        std::os::unix::fs::symlink(&example, directory.join(name)).unwrap();
    }
}

fn executable() -> PathBuf {
    let current = std::env::current_exe().unwrap();
    let examples = current
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("examples");
    std::fs::read_dir(&examples).unwrap()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name == "fake-ops-command" || name.strip_prefix("fake_ops_command-").is_some_and(|hash| {
                !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit())
            })
        })
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .max()
        .map_or_else(|| panic!("build native fixture first: cargo build -p baylee-ops --example fake-ops-command ({})", examples.display()), |(_, path)| path)
}
