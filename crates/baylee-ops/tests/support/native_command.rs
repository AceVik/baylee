//! Locate Cargo's non-shipped native command fixture, as the seat CLI tests do.
use std::path::{Path, PathBuf};

pub fn install(directory: &Path, names: &[&str]) {
    let example = executable();
    for name in names {
        std::os::unix::fs::symlink(&example, directory.join(name)).unwrap();
    }
}

/// The fixture, built beside the tests as an example: the plain binary, or
/// where `--all-targets` built it as a test (without the harness, so it is
/// the same program), the hashed one; the newest of them.
fn executable() -> PathBuf {
    let current = std::env::current_exe().unwrap();
    let dirs = example_dirs(&current);
    dirs.iter()
        .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten().flatten())
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name == "fake-ops-command"
                || name == "fake_ops_command"
                || name.strip_prefix("fake_ops_command-").is_some_and(|hash| {
                    !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit())
                })
        })
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .max()
        .map_or_else(
            || {
                panic!(
                    "build native fixture first: cargo build -p baylee-ops --example \
                     fake-ops-command (looked in {dirs:?})"
                )
            },
            |(_, path)| path,
        )
}

/// Where Cargo leaves this package's examples, found from the test's own
/// path. Up to Cargo 1.99 a test is `<profile>/deps/<test>` and its examples
/// lie in `<profile>/examples/`. Since 1.100 every unit builds in a directory
/// of its own, `<profile>/build/<package>/<hash>/out/`, and an example stays
/// there unless `cargo build --example` copies it up to `<profile>/examples/`
/// (nextest and `--all-targets` do not), so both are searched.
fn example_dirs(test: &Path) -> Vec<PathBuf> {
    let Some(dir) = test.parent() else {
        return Vec::new();
    };
    let profile = match dir.file_name().and_then(|name| name.to_str()) {
        Some("deps") => dir.parent(),
        // `out`, `<hash>`, `<package>`, `build`, then the profile.
        Some("out") => dir.ancestors().nth(4),
        _ => None,
    };
    let Some(profile) = profile else {
        return Vec::new();
    };
    let units = profile.join("build").join(env!("CARGO_PKG_NAME"));
    std::iter::once(profile.join("examples"))
        .chain(
            std::fs::read_dir(units)
                .into_iter()
                .flatten()
                .flatten()
                .map(|unit| unit.path().join("out")),
        )
        .collect()
}
