//! Which program is run, with what limits, and how it is found.

use super::{
    Arc, COMMON, CliTool, Dialect, Duration, OsString, Path, PathBuf, Settings, WINDOWS, agy,
    claude, cli_model, codex, is_absolute_path, junie, opencode, shaped_like_a_key,
};

/// Whether `name` is a variable no CLI is ever given, whatever asks for
/// it: a key or a token, the bridge's own, a forge's, a cloud's or a
/// provider's credentials, the SSH agent, a database.
#[must_use]
pub fn forbidden(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let prefixed = [
        "BAYLEE_",
        "GITHUB_",
        "GH_",
        "AWS_",
        "ANTHROPIC_",
        "OPENAI_",
        "GEMINI_",
        "GOOGLE_",
        "DEEPSEEK_",
    ]
    .iter()
    .any(|prefix| upper.starts_with(prefix));
    prefixed
        || upper.ends_with("_API_KEY")
        || upper.ends_with("_TOKEN")
        || upper.contains("SECRET")
        || upper.contains("PASSWORD")
        || matches!(upper.as_str(), "SSH_AUTH_SOCK" | "DATABASE_URL")
}

/// How long processes live and how many at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// A session idle this long is ended [300 s]: the five minutes
    /// Anthropic's API keeps a cached prefix by default. A conversation
    /// resumed after that is read whole again at the price of writing it to
    /// the cache, which costs more than the prefix and notes a new one
    /// begins with.
    pub idle: Duration,
    /// The most processes alive at once; starting one more ends the least
    /// recently used idle one [2].
    pub max_sessions: usize,
    /// How long a process whose stdin was closed gets to end before it is
    /// killed [2 s].
    pub grace: Duration,
    /// The first cooldown after a rate limit that names no time, doubling
    /// with each to fifteen minutes [60 s].
    pub cooldown: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            idle: Duration::from_secs(300),
            max_sessions: 2,
            grace: Duration::from_secs(2),
            cooldown: Duration::from_secs(60),
        }
    }
}

/// What a CLI mind starts its processes with, checked before a game
/// reserves anything: the tool, its program, its own model, and the
/// parent's variables it is given.
#[derive(Clone)]
pub struct Launch {
    pub(super) dialect: Arc<dyn Dialect>,
    pub(super) program: PathBuf,
    pub(super) model: Option<String>,
    pub(super) passed: Vec<(&'static str, String)>,
}

impl std::fmt::Debug for Launch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.passed.iter().map(|(name, _)| *name).collect();
        f.debug_struct("Launch")
            .field("tool", &self.dialect.tool())
            .field("program", &self.program)
            .field("model", &self.model)
            .field("passed", &names)
            .finish()
    }
}

impl Launch {
    /// The tool `settings.model` names (`claude:opus`), its program at
    /// `command` or else found on the `PATH` that `env` gives, and the
    /// variables `env` gives it.
    ///
    /// # Errors
    /// A sentence: a tool this build does not speak, a command that is not
    /// a whole path to a program this user may run, a tool not on `PATH`,
    /// or a passed variable that looks like a key (named, never shown).
    pub fn new(
        settings: &Settings,
        command: Option<&str>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let (tool, model) = cli_model(&settings.model)?;
        let dialect = dialect(tool);
        let program = program(tool, command, env)?;
        let passed = COMMON
            .iter()
            .chain(dialect.passed_env())
            .chain(WINDOWS.iter())
            .filter_map(|name| {
                env(name)
                    .filter(|value| !value.is_empty())
                    .map(|value| (*name, value))
            })
            .collect();
        let launch = Self {
            dialect,
            program,
            model: model.map(str::to_string),
            passed,
        };
        let temp = std::env::temp_dir();
        launch.env(&temp, &temp, Some(&temp))?;
        Ok(launch)
    }

    /// The value the parent gives the tool for `name`, of those it passes.
    pub(super) fn passed(&self, name: &str) -> Option<OsString> {
        self.passed
            .iter()
            .find(|(passed, _)| *passed == name)
            .map(|(_, value)| value.into())
    }

    /// Where the tool keeps its sessions outside the seat's store
    /// ([`Dialect::sessions_root`]).
    pub(super) fn sessions_root(&self) -> Option<PathBuf> {
        self.dialect.sessions_root(&|name| self.passed(name))
    }

    /// The tool.
    #[must_use]
    pub fn tool(&self) -> CliTool {
        self.dialect.tool()
    }

    /// The program it runs.
    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// A process's whole environment, with `tmp` as its temp directory,
    /// `support` the directory of its session's own files and `store` the
    /// seat's store of a tool that resumes ([`Dialect::store_env`]).
    ///
    /// # Errors
    /// For a variable no CLI is given, by its name ([`forbidden`]), and
    /// for one whose value looks like a key.
    pub(super) fn env(
        &self,
        tmp: &Path,
        support: &Path,
        store: Option<&Path>,
    ) -> Result<Vec<(String, OsString)>, String> {
        let mut env: Vec<(String, OsString)> = self
            .passed
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.into()))
            .collect();
        let temp = if cfg!(windows) {
            &["TMPDIR", "TEMP", "TMP"][..]
        } else {
            &["TMPDIR"][..]
        };
        env.extend(temp.iter().map(|name| ((*name).to_string(), tmp.into())));
        let fixed = [
            ("LANG", "C.UTF-8"),
            ("LC_ALL", "C.UTF-8"),
            ("TERM", "dumb"),
            ("NO_COLOR", "1"),
        ];
        for (name, value) in fixed.iter().chain(self.dialect.fixed_env()) {
            env.push(((*name).into(), (*value).into()));
        }
        for (name, value) in self.dialect.session_env(support) {
            env.push((name.into(), value));
        }
        if let Some(store) = store {
            for (name, value) in self.dialect.store_env(store) {
                env.push((name.into(), value));
            }
        }
        let tool = self.dialect.tool().name();
        for (name, value) in &env {
            if forbidden(name) {
                return Err(format!(
                    "{name} is never given to a CLI, and {tool} does not start"
                ));
            }
            if shaped_like_a_key(&value.to_string_lossy()) {
                return Err(format!(
                    "{name} looks like a key, and a key is never given to a CLI: {tool} does not \
                     start"
                ));
            }
        }
        Ok(env)
    }
}

/// The program for `tool`: `command`, which must be a whole path, else the
/// first `tool` on the `PATH` that `env` gives, in its absolute entries
/// only. Never canonicalised: a link on `PATH` runs as the link.
pub(super) fn program(
    tool: CliTool,
    command: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<PathBuf, String> {
    let name = tool.name();
    if let Some(command) = command {
        if !is_absolute_path(command) {
            return Err(format!(
                "a profile's command is a whole path to {name}, such as /opt/homebrew/bin/{name}"
            ));
        }
        let path = PathBuf::from(command);
        return if runnable(&path) {
            Ok(path)
        } else {
            Err(format!(
                "{} is not a program this user may run",
                path.display()
            ))
        };
    }
    let path = env("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)))
        .find(|candidate| runnable(candidate))
        .ok_or_else(|| {
            format!("{name} is not on PATH: install it, or name its program in a profile's command")
        })
}

/// Whether `path` is a file this user may run, through any links.
pub(super) fn runnable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && executable(&meta))
}

#[cfg(unix)]
pub(super) fn executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
pub(super) fn executable(_: &std::fs::Metadata) -> bool {
    true
}

/// The dialect of `tool`.
pub(super) fn dialect(tool: CliTool) -> Arc<dyn Dialect> {
    match tool {
        CliTool::Claude => Arc::new(claude::Claude),
        CliTool::Agy => Arc::new(agy::Agy),
        CliTool::Codex => Arc::new(codex::Codex),
        CliTool::Opencode => Arc::new(opencode::Opencode),
        CliTool::Junie => Arc::new(junie::Junie),
    }
}
