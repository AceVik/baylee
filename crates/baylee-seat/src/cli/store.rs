//! The working directories a CLI runs in and the session stores it leaves,
//! and sweeping them.

use super::{CliTool, Dialect, Duration, Path, PathBuf, dialect};

/// A session's own directory under the OS's temp directory, removed with
/// it: `work`, the process's working directory, empty; `tmp`, its temp
/// directory; and `support`, the files its dialect points it at
/// ([`Dialect::files`]); all four readable by this user alone.
pub(super) struct SessionDir {
    pub(super) root: PathBuf,
}

impl SessionDir {
    pub(super) fn new(game: &str, seat: u8) -> std::io::Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "baylee-cli-{}-{seat}-{}",
            tag(game),
            uuid::Uuid::now_v7().simple()
        ));
        private_dir(&root)?;
        let dir = Self { root };
        private_dir(&dir.work())?;
        private_dir(&dir.tmp())?;
        private_dir(&dir.support())?;
        Ok(dir)
    }

    pub(super) fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    pub(super) fn tmp(&self) -> PathBuf {
        self.root.join("tmp")
    }

    pub(super) fn support(&self) -> PathBuf {
        self.root.join("support")
    }

    /// Writes `files` into `support`, each readable by this user alone.
    pub(super) fn write(&self, files: &[(&'static str, String)]) -> std::io::Result<()> {
        for (name, contents) in files {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(self.support().join(name))?;
            std::io::Write::write_all(&mut file, contents.as_bytes())?;
        }
        Ok(())
    }
}

impl Drop for SessionDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Makes `path`, which must not exist, readable by this user alone.
pub(super) fn private_dir(path: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

/// What a seat's [`Store`] is called under the OS's temp directory, before
/// the game, the seat and a unique tail.
pub(super) const STORE_PREFIX: &str = "baylee-cli-store-";

/// A [`Store`] untouched this long belongs to no bridge still playing it
/// (each process started in it touches it, and a conversation idle for
/// [`Limits::idle`] is over), and the next mind to start sweeps it away.
pub(super) const STALE_STORE: Duration = Duration::from_hours(1);

/// A seat's conversation kept on disk for a tool that resumes one
/// ([`Dialect::resumes`]): under the OS's temp directory beside the
/// sessions' own directories, readable by this user alone, removed with
/// the conversation (and so with the seat and the mind); `work`, the
/// working directory every process of the conversation shares, empty, and
/// `data`, where the tool keeps the conversation ([`Dialect::store_env`]).
/// Nothing of the user's own sessions is read or written: the tool is
/// pointed only here.
pub(super) struct Store {
    pub(super) root: PathBuf,
}

impl Store {
    pub(super) fn new(game: &str, seat: u8) -> std::io::Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "{STORE_PREFIX}{}-{seat}-{}",
            tag(game),
            uuid::Uuid::now_v7().simple()
        ));
        private_dir(&root)?;
        let store = Self { root };
        private_dir(&store.work())?;
        private_dir(&store.data())?;
        store.touch();
        Ok(store)
    }

    pub(super) fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    pub(super) fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    /// Marks the store as in use now, for [`sweep_stores`].
    pub(super) fn touch(&self) {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let _ = options.open(self.root.join("used"));
    }

    /// Writes down which of `tool`'s sessions under `sessions` are the
    /// conversation's (`ids`), for the sweep, should the bridge be killed
    /// before it removes them itself.
    pub(super) fn record(&self, tool: CliTool, sessions: &Path, ids: &[String]) {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut lines = vec![tool.name().to_string(), sessions.display().to_string()];
        lines.extend(ids.iter().cloned());
        if let Ok(mut file) = options.open(self.root.join(SESSIONS)) {
            let _ = std::io::Write::write_all(&mut file, lines.join("\n").as_bytes());
        }
    }
}

/// The file in a [`Store`] that names the conversation's sessions kept
/// outside it ([`Store::record`]).
pub(super) const SESSIONS: &str = "sessions";

/// Removes what [`Dialect::session_files`] names as the conversation `id`
/// under `sessions`, and nothing else. How many went.
pub(super) fn forget(dialect: &dyn Dialect, sessions: &Path, id: &str) -> usize {
    let mut gone = 0;
    for path in dialect.session_files(sessions, id) {
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let removed = if meta.is_dir() {
            std::fs::remove_dir_all(&path)
        } else if meta.is_file() {
            std::fs::remove_file(&path)
        } else {
            continue;
        };
        gone += usize::from(removed.is_ok());
    }
    gone
}

/// Removes the sessions a stale store names ([`Store::record`]): its
/// tool's own, under the directory it names, by the tool's own matching.
pub(super) fn forget_recorded(store: &Path) {
    let Ok(text) = std::fs::read_to_string(store.join(SESSIONS)) else {
        return;
    };
    let mut lines = text.lines();
    let (Some(tool), Some(sessions)) = (lines.next().and_then(CliTool::named), lines.next()) else {
        return;
    };
    let sessions = Path::new(sessions);
    if !sessions.is_absolute() {
        return;
    }
    let dialect = dialect(tool);
    for id in lines {
        forget(dialect.as_ref(), sessions, id);
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Removes the [`Store`]s in `dir` untouched for `stale`: a bridge that
/// was killed left them. Only directories named as stores and private as
/// a store is are looked at, never through a link (another user's could
/// not be removed anyway). How many went.
pub(super) fn sweep_stores(dir: &Path, stale: Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut swept = 0;
    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(STORE_PREFIX))
        {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_dir() || !private(&meta) {
            continue;
        }
        let used = std::fs::symlink_metadata(path.join("used"))
            .and_then(|used| used.modified())
            .or_else(|_| meta.modified());
        let old = used.is_ok_and(|used| used.elapsed().is_ok_and(|age| age >= stale));
        if !old {
            continue;
        }
        forget_recorded(&path);
        if std::fs::remove_dir_all(&path).is_ok() {
            swept += 1;
        }
    }
    swept
}

/// Whether what `meta` describes is readable by its owner alone, as every
/// directory [`private_dir`] makes is.
#[cfg(unix)]
pub(super) fn private(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o777 == 0o700
}

#[cfg(not(unix))]
pub(super) fn private(_: &std::fs::Metadata) -> bool {
    true
}

/// `game` as a directory name's part: its letters, digits and `-`.
pub(super) fn tag(game: &str) -> String {
    game.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(24)
        .collect()
}
