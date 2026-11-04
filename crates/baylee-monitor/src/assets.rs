//! The files compiled into the binary: the page it serves, and its own source
//! for the footer's download. Nothing is read from disk at run time.

/// Files of this crate by their path relative to it.
macro_rules! embedded {
    ($($path:literal),+ $(,)?) => {
        &[$(($path, include_str!(concat!("../", $path)))),+]
    };
}

/// The page, served at its path below `web/`.
pub(crate) const WEB: &[(&str, &str)] = embedded![
    "web/index.html",
    "web/style.css",
    "web/js/alerts.js",
    "web/js/cards.js",
    "web/js/chart.js",
    "web/js/data.js",
    "web/js/disks.js",
    "web/js/dom.js",
    "web/js/format.js",
    "web/js/layout.js",
    "web/js/main.js",
    "web/js/navigation.js",
    "web/js/performance.js",
    "web/js/processes.js",
    "web/js/settings.js",
    "web/js/store.js",
    "web/js/theme-early.js",
    "web/js/theme.js",
    "web/js/training.js",
];

const CRATE: &[(&str, &str)] = embedded![
    "README.md",
    "Cargo.toml",
    "src/admission.rs",
    "src/assets.rs",
    "src/background.rs",
    "src/gpu.rs",
    "src/history.rs",
    "src/listen.rs",
    "src/main.rs",
    "src/network.rs",
    "src/power.rs",
    "src/processes.rs",
    "src/procfs/diskstats.rs",
    "src/procfs/meminfo.rs",
    "src/procfs/mod.rs",
    "src/procfs/mountinfo.rs",
    "src/procfs/netdev.rs",
    "src/procfs/pressure.rs",
    "src/procfs/sockstat.rs",
    "src/procfs/stat.rs",
    "src/procfs/status.rs",
    "src/procfs/swaps.rs",
    "src/procfs/vmstat.rs",
    "src/sampler.rs",
    "src/sensors.rs",
    "src/server.rs",
    "src/storage/inventory.rs",
    "src/storage/mod.rs",
    "src/storage/rates.rs",
    "src/storage/udev.rs",
    "src/storage/usage.rs",
    "src/test_support.rs",
    "src/training/dbus.rs",
    "src/training/mod.rs",
    "src/training/pipeline.rs",
    "src/training/units.rs",
    "src/types.rs",
];

const WORKSPACE: &[(&str, &str)] = &[
    ("workspace-Cargo.toml", include_str!("../../../Cargo.toml")),
    ("Cargo.lock", include_str!("../../../Cargo.lock")),
    ("LICENSE", include_str!("../../../LICENSE")),
];

/// Every file of the source download, by its name there.
pub(crate) fn source_files() -> impl Iterator<Item = (&'static str, &'static str)> {
    WORKSPACE.iter().chain(CRATE).chain(WEB).copied()
}

/// The page's scripts, by their path.
#[cfg(test)]
pub(crate) fn scripts() -> impl Iterator<Item = (&'static str, &'static str)> {
    WEB.iter()
        .copied()
        .filter(|(path, _)| served(path).1.starts_with("text/javascript"))
}

/// The path a page file is served at and its content type.
#[allow(clippy::case_sensitive_file_extension_comparisons)] // Our own file names.
pub(crate) fn served(path: &str) -> (&str, &'static str) {
    let url = path.strip_prefix("web").unwrap_or(path);
    let kind = if url.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if url.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else {
        "text/html; charset=utf-8"
    };
    (url, kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeSet, fs, path::Path};

    fn walk(root: &Path, directory: &Path, found: &mut BTreeSet<String>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, found);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                found.insert(relative.to_string_lossy().into_owned());
            }
        }
    }

    /// The download must rebuild, and a page file nobody serves is a dead link:
    /// so a new file in `src/` or `web/` has to be listed here.
    #[test]
    fn every_source_and_page_file_is_embedded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut on_disk = BTreeSet::new();
        walk(root, &root.join("src"), &mut on_disk);
        walk(root, &root.join("web"), &mut on_disk);
        let listed: BTreeSet<String> = source_files().map(|(name, _)| name.to_owned()).collect();
        let missing: Vec<_> = on_disk.difference(&listed).collect();
        assert!(missing.is_empty(), "not embedded: {missing:?}");
    }

    #[test]
    fn every_module_the_page_imports_is_served() {
        for (path, text) in scripts() {
            for line in text.lines().filter(|line| line.starts_with("import ")) {
                let module = line.rsplit('"').nth(1).unwrap();
                let wanted = format!("web/js/{}", module.trim_start_matches("./"));
                assert!(
                    WEB.iter().any(|(name, _)| *name == wanted),
                    "{path} imports {module}, which is not served"
                );
            }
        }
    }

    #[test]
    fn page_files_get_their_paths_and_types() {
        assert_eq!(
            served("web/js/main.js"),
            ("/js/main.js", "text/javascript; charset=utf-8")
        );
        assert_eq!(
            served("web/style.css"),
            ("/style.css", "text/css; charset=utf-8")
        );
    }
}
