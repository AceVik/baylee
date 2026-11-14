//! Strict standalone commands and authenticated attachment configuration.
/// What `baylee-engine-server --help` prints, and a refused command line
/// after its reason.
pub(super) const USAGE: &str = "\
usage: baylee-engine-server --attach <url> --game <id>
           play one game for a gateway, as an agent starts it; the game's
           token is in BAYLEE_ENGINE_TOKEN (--token <token> is read too, but
           an argument is in the process list for every user of the machine)
       baylee-engine-server
           the dev harness on 127.0.0.1:28765 (PORT, BAYLEE_BIND): no
           authentication, every hand to whoever asks; never bind it publicly
       baylee-engine-server --help | -h
           this text
       baylee-engine-server --metadata
           one bounded JSON execution identity; no services or credentials

BAYLEE_ATTACH_URL, BAYLEE_GAME and BAYLEE_ENGINE_TOKEN stand in for the
three flags; an attachment takes all three or none.";

/// What a command line asks for, read before anything else is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Asked {
    /// Read only the compiled execution identity.
    Metadata,
    /// `-h` or `--help`: print [`USAGE`], start nothing.
    Help,
    /// Start: attached when [`Attach::read`] finds an attachment, the dev
    /// harness when the launch names none.
    Start,
}

impl Asked {
    /// The flags an attachment is named with, each followed by its value.
    const FLAGS: [&'static str; 3] = ["--attach", "--game", "--token"];

    /// Reads a command line's shape: the three flags, each at most once and
    /// each with a value, or a request for help.
    ///
    /// A refusal names a flag and never a value: a value may be a token.
    ///
    /// # Errors
    /// Any other argument, a flag given twice, or a flag with no value.
    pub(super) fn read(args: &[String]) -> Result<Self, String> {
        if args == ["--metadata"] {
            return Ok(Self::Metadata);
        }
        let mut seen: Vec<&str> = Vec::new();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let arg = arg.as_str();
            if arg == "-h" || arg == "--help" {
                return Ok(Self::Help);
            }
            let Some(flag) = Self::FLAGS.into_iter().find(|flag| *flag == arg) else {
                return Err(if arg.starts_with('-') {
                    let name = arg.split_once('=').map_or(arg, |(name, _)| name);
                    format!("{name:?} is not a flag this program takes")
                } else {
                    "a value with no flag before it".to_string()
                });
            };
            if seen.contains(&flag) {
                return Err(format!("{flag} is given twice"));
            }
            seen.push(flag);
            if rest.next().is_none_or(|value| value.starts_with('-')) {
                return Err(format!("{flag} takes a value"));
            }
        }
        Ok(Self::Start)
    }
}

/// Where an attached engine dials, and what it proves it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attach {
    /// The gateway's engine socket (`ws://…/engine/ws`), or its unix socket
    /// (`unix:<path>`), on which the engine plane is at `/engine/ws`.
    pub url: String,
    /// The game this process was started for.
    pub game_id: String,
    /// One game's worth of authority, issued by the gateway and handed over
    /// by the agent. It never leaves the machine the agent runs on.
    pub token: String,
}

/// A launch that names an attachment and leaves part of it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialAttach {
    /// What it would need: the flag of each value it has none for.
    pub(super) missing: Vec<&'static str>,
}

impl std::fmt::Display for PartialAttach {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "an attachment takes all of --attach, --game and --token and this launch has no {}; \
             give all three, or none for the dev harness",
            self.missing.join(", ")
        )
    }
}

impl Attach {
    /// The attachment this launch was given, if it was given one.
    ///
    /// Command line first (that is how the agent starts it), environment
    /// second (that is how a person starts it by hand).
    ///
    /// # Errors
    /// [`PartialAttach`], as [`Attach::read`].
    pub(super) fn discover(args: &[String]) -> Result<Option<Self>, PartialAttach> {
        Self::read(args, |name| std::env::var(name).ok())
    }

    /// The attachment a given command line and environment describe.
    ///
    /// Split from [`Attach::discover`] so that the precedence and the
    /// all-or-nothing rule can be asserted: a function that read the process
    /// arguments itself could not be tested for what it does with them, which
    /// is the same bargain `import::plan` makes with the clock.
    ///
    /// The all-or-nothing half is the one worth having a test for. `None` is
    /// [`super::listen`], the dev harness, which has no authentication at all and
    /// hands a socket whichever seat it names, so only a launch that says
    /// nothing about an attachment gets it. A launch that says anything and
    /// leaves the rest out was meant to attach, and it stops.
    ///
    /// # Errors
    /// [`PartialAttach`] when some of the three are named and not all.
    pub(super) fn read(
        args: &[String],
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, PartialAttach> {
        let flag = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        // An empty variable is no value. A shell that exports
        // `BAYLEE_ENGINE_TOKEN=` from an unset value would otherwise attach
        // with an empty token and be refused by the gateway, which reads as
        // a rejected engine rather than as a launch that was never given one.
        // It still says that this launch was meant to attach.
        let from_env = |name: &str| env(name).filter(|v| !v.is_empty());
        let named = |flag: &str, var: &str| args.iter().any(|a| a == flag) || env(var).is_some();
        let url = flag("--attach").or_else(|| from_env("BAYLEE_ATTACH_URL"));
        let game_id = flag("--game").or_else(|| from_env("BAYLEE_GAME"));
        let token = flag("--token").or_else(|| from_env("BAYLEE_ENGINE_TOKEN"));
        if let (Some(url), Some(game_id), Some(token)) = (&url, &game_id, &token) {
            return Ok(Some(Self {
                url: url.clone(),
                game_id: game_id.clone(),
                token: token.clone(),
            }));
        }
        let said_anything = named("--attach", "BAYLEE_ATTACH_URL")
            || named("--game", "BAYLEE_GAME")
            || named("--token", "BAYLEE_ENGINE_TOKEN");
        if !said_anything {
            return Ok(None);
        }
        let missing = [
            ("--attach", url.is_none()),
            ("--game", game_id.is_none()),
            ("--token", token.is_none()),
        ]
        .into_iter()
        .filter_map(|(flag, absent)| absent.then_some(flag))
        .collect();
        Err(PartialAttach { missing })
    }
}
