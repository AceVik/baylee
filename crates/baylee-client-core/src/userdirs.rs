//! Where the client keeps a player's files, per operating system.
//!
//! One pure function over the operating system and an environment lookup,
//! so every platform's answer is tested on whichever machine runs the tests:
//! CI's Linux runner checks the Windows answer as surely as a Windows one
//! would. The client asks it with [`Os::current`] and the real environment
//! (`settings::store`, `artreader::cache_home`).
//!
//! Until #324 the settings read only `XDG_CONFIG_HOME` and `HOME`. Windows
//! normally sets neither, so a Windows client saved nothing: no settings,
//! no kept guest, no saved gateway, no report consent, no crash report.
//! Windows now answers `%APPDATA%\Baylee`, and every other system answers
//! exactly what it did before, so no player's existing files move.

use std::ffi::OsString;
use std::path::PathBuf;

/// The operating systems whose conventions differ here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// macOS and iOS (`HOME` is an iOS app's own container).
    Apple,
    /// Windows.
    Windows,
    /// Android: no environment names its directories. A `NativeActivity` is
    /// handed its files path instead, which only the client can ask for.
    Android,
    /// Linux and the other unixes.
    Other,
}

impl Os {
    /// The system this was built for.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_vendor = "apple") {
            Self::Apple
        } else if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "android") {
            Self::Android
        } else {
            Self::Other
        }
    }
}

/// What a directory is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// What must survive: settings, kept guests, hand-built decks, a crash
    /// waiting to be reported.
    Config,
    /// What may be thrown away and fetched again: card art.
    Cache,
}

/// The client's own directory of `kind` on `os`, reading the environment
/// through `env`, or `None` where the environment names none.
///
/// A variable set to the empty string counts as unset, as the XDG spec says
/// and as a shell's `VAR= command` means.
///
/// - **Config**: `$XDG_CONFIG_HOME/baylee` wherever it is set, which is also
///   how a live check points a client at a scratch copy. Then on Windows
///   `%APPDATA%\Baylee`, else `%USERPROFILE%\AppData\Roaming\Baylee`, where
///   `%APPDATA%` lives by default. Everywhere else, Android included,
///   `$HOME/.config/baylee`, as before.
/// - **Cache**: `$XDG_CACHE_HOME/baylee` when it is set and absolute (the
///   spec ignores a relative one), else `~/Library/Caches/baylee` on Apple,
///   `%LOCALAPPDATA%\baylee` on Windows, `~/.cache/baylee` on the other
///   unixes. On Android, `None`: the client takes the cache beside the files
///   path it is handed.
#[must_use]
pub fn user_dir(kind: Kind, os: Os, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let set = |key: &str| {
        env(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    match kind {
        Kind::Config => {
            if let Some(xdg) = set("XDG_CONFIG_HOME") {
                return Some(xdg.join("baylee"));
            }
            match os {
                Os::Windows => set("APPDATA")
                    .or_else(|| set("USERPROFILE").map(|home| home.join("AppData").join("Roaming")))
                    .map(|dir| dir.join("Baylee")),
                Os::Apple | Os::Android | Os::Other => {
                    set("HOME").map(|home| home.join(".config").join("baylee"))
                }
            }
        }
        Kind::Cache => {
            if let Some(xdg) = set("XDG_CACHE_HOME").filter(|dir| dir.is_absolute()) {
                return Some(xdg.join("baylee"));
            }
            let base = match os {
                Os::Apple => set("HOME").map(|home| home.join("Library").join("Caches")),
                Os::Windows => set("LOCALAPPDATA"),
                Os::Other => set("HOME").map(|home| home.join(".cache")),
                Os::Android => None,
            };
            base.map(|dir| dir.join("baylee"))
        }
    }
}

/// The process's own environment, as [`user_dir`] reads it.
#[must_use]
pub fn real_env(key: &str) -> Option<OsString> {
    std::env::var_os(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An environment of exactly these variables.
    fn env(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        move |key| {
            vars.iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| OsString::from(*v))
        }
    }

    fn config(os: Os, vars: &'static [(&'static str, &'static str)]) -> Option<PathBuf> {
        user_dir(Kind::Config, os, &env(vars))
    }

    fn cache(os: Os, vars: &'static [(&'static str, &'static str)]) -> Option<PathBuf> {
        user_dir(Kind::Cache, os, &env(vars))
    }

    /// The fault (#324): a Windows machine sets `APPDATA` and
    /// `USERPROFILE` and neither `HOME` nor `XDG_CONFIG_HOME`, and the client
    /// saved nothing there.
    #[test]
    fn windows_keeps_its_settings_in_appdata() {
        const WINDOWS: &[(&str, &str)] = &[
            ("APPDATA", r"C:\Users\ada\AppData\Roaming"),
            ("USERPROFILE", r"C:\Users\ada"),
            ("LOCALAPPDATA", r"C:\Users\ada\AppData\Local"),
        ];
        assert_eq!(
            config(Os::Windows, WINDOWS),
            Some(PathBuf::from(r"C:\Users\ada\AppData\Roaming").join("Baylee"))
        );
        // Without `APPDATA`, where it lives by default.
        assert_eq!(
            config(Os::Windows, &[("USERPROFILE", r"C:\Users\ada")]),
            Some(
                PathBuf::from(r"C:\Users\ada")
                    .join("AppData")
                    .join("Roaming")
                    .join("Baylee")
            )
        );
        // A `HOME` a Git Bash sets does not take it back to `.config`.
        assert_eq!(
            config(
                Os::Windows,
                &[("APPDATA", r"C:\R"), ("HOME", "/c/Users/ada")]
            ),
            Some(PathBuf::from(r"C:\R").join("Baylee"))
        );
        // Nothing to go on is nothing, not the working directory.
        assert_eq!(config(Os::Windows, &[]), None);
        assert_eq!(
            config(Os::Windows, &[("APPDATA", ""), ("USERPROFILE", "")]),
            None
        );
    }

    /// macOS, Linux and iOS keep the directory they always had, so no
    /// player's settings move: `XDG_CONFIG_HOME` first, then `~/.config`.
    #[test]
    fn the_unixes_keep_the_directory_they_had() {
        for os in [Os::Apple, Os::Other, Os::Android] {
            assert_eq!(
                config(os, &[("HOME", "/home/ada")]),
                Some(PathBuf::from("/home/ada/.config/baylee")),
                "{os:?}"
            );
            assert_eq!(
                config(os, &[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "/x/cfg")]),
                Some(PathBuf::from("/x/cfg/baylee")),
                "{os:?}"
            );
            // An empty `XDG_CONFIG_HOME` is unset.
            assert_eq!(
                config(os, &[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "")]),
                Some(PathBuf::from("/home/ada/.config/baylee")),
                "{os:?}"
            );
            // Windows' variables mean nothing here.
            assert_eq!(config(os, &[("APPDATA", "/a")]), None, "{os:?}");
        }
    }

    /// `XDG_CONFIG_HOME` wins on Windows too: it is how a live check points
    /// a client at a scratch copy of the settings, on any machine.
    #[test]
    fn an_explicit_config_home_wins_everywhere() {
        assert_eq!(
            config(
                Os::Windows,
                &[("XDG_CONFIG_HOME", "/x/cfg"), ("APPDATA", r"C:\R")]
            ),
            Some(PathBuf::from("/x/cfg/baylee"))
        );
    }

    /// The cache answers what `artreader::cache_home` answered before it
    /// asked here.
    #[test]
    fn the_cache_is_where_it_was() {
        assert_eq!(
            cache(Os::Apple, &[("HOME", "/Users/ada")]),
            Some(PathBuf::from("/Users/ada/Library/Caches/baylee"))
        );
        assert_eq!(
            cache(Os::Other, &[("HOME", "/home/ada")]),
            Some(PathBuf::from("/home/ada/.cache/baylee"))
        );
        assert_eq!(
            cache(
                Os::Windows,
                &[("LOCALAPPDATA", r"C:\L"), ("HOME", "/c/ada")]
            ),
            Some(PathBuf::from(r"C:\L").join("baylee"))
        );
        assert_eq!(cache(Os::Android, &[("HOME", "/data")]), None);
        // An absolute `XDG_CACHE_HOME` wins; a relative one is ignored.
        assert_eq!(
            cache(
                Os::Other,
                &[("HOME", "/home/ada"), ("XDG_CACHE_HOME", "/x/cache")]
            ),
            Some(PathBuf::from("/x/cache/baylee"))
        );
        assert_eq!(
            cache(
                Os::Other,
                &[("HOME", "/home/ada"), ("XDG_CACHE_HOME", "cache")]
            ),
            Some(PathBuf::from("/home/ada/.cache/baylee"))
        );
    }

    /// The system this was built for is the one [`Os::current`] names.
    #[test]
    fn the_current_os_is_the_target_s() {
        let os = Os::current();
        if cfg!(target_os = "macos") {
            assert_eq!(os, Os::Apple);
        } else if cfg!(windows) {
            assert_eq!(os, Os::Windows);
        } else if cfg!(target_os = "linux") {
            assert_eq!(os, Os::Other);
        }
    }
}
