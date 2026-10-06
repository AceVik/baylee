//! What a language model plays with: the settings file
//! ([`baylee_client_core::llmseat`], `docs/llm-seat.md`) and the command
//! line over it.
//!
//! A flag overrides the chosen profile, which overrides the build's
//! defaults. The chosen profile is `--profile <name>`, else the file's
//! default. The model is `--mind`'s where it names one, else the profile's;
//! `--mind` over a profile of another provider is refused when the profile
//! was named, and plays with the build's defaults when it was only the
//! default. With no file at all the bridge plays as it always did: no
//! profile, no caps, no spend book.

use crate::llm::{AnswerMode, Price, Provider, Settings, Spec};
use baylee_client_core::llmseat::ledger::Book;
use baylee_client_core::llmseat::{Profile, SeatSettings, effort_is_a_word, store};
use baylee_client_core::userdirs::Os;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The environment variable that names the settings file, under
/// `--config`.
pub const CONFIG_ENV: &str = store::CONFIG_ENV;

/// Where the bridge's settings file and spend book are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// The settings file, where one can be named at all.
    pub settings: Option<PathBuf>,
    /// Whether it was named (`--config`, [`CONFIG_ENV`]): a file named
    /// must be there, where the default one may be missing.
    pub named: bool,
    /// The spend book named by `--ledger`, if one was.
    pub ledger: Option<PathBuf>,
}

impl Paths {
    /// `config` (`--config`), else [`CONFIG_ENV`], else the client's config
    /// directory ([`store::default_path`]), with `env` as the environment:
    /// a test hands it a table and never reaches the player's own files.
    #[must_use]
    pub fn resolve(
        config: Option<&Path>,
        ledger: Option<&Path>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Self {
        let named = config
            .map(Path::to_path_buf)
            .or_else(|| env(CONFIG_ENV).filter(|v| !v.is_empty()).map(PathBuf::from));
        let named_one = named.is_some();
        let settings =
            named.or_else(|| store::default_path(Os::current(), &|k| env(k).map(OsString::from)));
        Self {
            settings,
            named: named_one,
            ledger: ledger.map(Path::to_path_buf),
        }
    }

    /// The settings file, or `None` where there is none to read: the
    /// default one missing, or no config directory at all.
    ///
    /// # Errors
    /// For a named file that is not there, and for any file that cannot be
    /// read or is refused ([`SeatSettings::parse`]).
    pub fn load(&self) -> Result<Option<SeatSettings>, String> {
        let Some(path) = &self.settings else {
            return Ok(None);
        };
        match store::load(path)? {
            None if self.named => Err(format!(
                "there is no settings file at {} (named by --config or {CONFIG_ENV})",
                path.display()
            )),
            loaded => Ok(loaded),
        }
    }

    /// The spend book a game is counted in: the one `--ledger` names, else
    /// the one beside a settings file that was read; `None` with neither,
    /// as without a file.
    #[must_use]
    pub fn book(&self, file_read: bool) -> Option<Book> {
        match (&self.ledger, &self.settings) {
            (Some(ledger), _) => Some(Book::new(ledger.clone())),
            (None, Some(settings)) if file_read => Some(Book::beside(settings)),
            _ => None,
        }
    }

    /// The settings file, for a sentence.
    #[must_use]
    pub fn named_file(&self) -> String {
        self.settings.as_ref().map_or_else(
            || "this machine's config directory (none is set)".into(),
            |path| path.display().to_string(),
        )
    }
}

/// What the command line says over a profile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    /// `--effort`.
    pub effort: Option<String>,
    /// `--default-effort`: the build's default effort, whatever the
    /// profile names (a chair that plays a model which does not take the
    /// profile's).
    pub default_effort: bool,
    /// `--answer`.
    pub answer: Option<AnswerMode>,
    /// `--max-tokens`.
    pub max_tokens: Option<u32>,
    /// `--price-in` and `--price-out`.
    pub price: Option<Price>,
    /// `--spend-usd`.
    pub spend_usd: Option<f64>,
    /// `--spend-tokens`.
    pub spend_tokens: Option<u64>,
    /// `--spend-calls`.
    pub spend_calls: Option<u64>,
    /// `--think-secs`.
    pub think_secs: Option<u64>,
}

/// A language model to play, and everything it plays with.
#[derive(Clone, Debug)]
pub struct Plan {
    /// The model.
    pub spec: Spec,
    /// How it plays and what a game may spend.
    pub settings: Settings,
    /// The longest one answer may take, in seconds.
    pub think_secs: u64,
    /// The environment variable its key is read from; `None` for a CLI,
    /// which reads none.
    pub key_env: Option<String>,
    /// Where the API is, when a profile names it.
    pub base_url: Option<String>,
    /// A CLI's program, when a profile names it: an absolute path. `None`
    /// finds the tool's name on `PATH`.
    pub command: Option<String>,
    /// The profile it plays, if one.
    pub profile: Option<String>,
    /// Something the player should be told: a default profile that did
    /// not apply.
    pub note: Option<String>,
}

/// A profile chosen, and whether it was named.
struct Chosen<'a> {
    name: &'a str,
    profile: &'a Profile,
    named: bool,
}

/// The language model to play: `mind` (`--mind`, when it names a model),
/// the profile named (`--profile`) or the file's default, and `flags` over
/// them; `None` when nothing names a model, and the house plays.
///
/// # Errors
/// A sentence: a profile named with no file, or one the file does not
/// have; `--mind` naming another provider than a named profile; a flag
/// that is not a value ([`Overrides`]); and [`Settings::budget`]'s
/// refusals (a model with no price and no token budget).
pub fn plan(
    mind: Option<&Spec>,
    file: Option<&SeatSettings>,
    paths: &Paths,
    named: Option<&str>,
    flags: &Overrides,
) -> Result<Option<Plan>, String> {
    let chosen = pick(file, paths, named)?;
    let mut note = None;
    let (spec, chosen) = match (mind, chosen) {
        (Some(spec), Some(chosen)) if chosen.profile.provider != spec.provider => {
            let (theirs, ours) = (chosen.profile.provider.name(), spec.provider.name());
            if chosen.named {
                return Err(format!(
                    "--mind names {ours}:{}, and profile «{}» plays {theirs}: leave out --mind, \
                     or name a profile of {ours}",
                    spec.model, chosen.name
                ));
            }
            note = Some(format!(
                "the settings file's default profile «{}» plays {theirs}, so {ours}:{} plays \
                 with this build's defaults",
                chosen.name, spec.model
            ));
            (spec.clone(), None)
        }
        (Some(spec), chosen) => (spec.clone(), chosen),
        (None, Some(chosen)) => (
            Spec {
                provider: chosen.profile.provider,
                model: chosen.profile.model.clone(),
            },
            Some(chosen),
        ),
        (None, None) => return Ok(None),
    };
    let settings = tune(&spec, chosen.as_ref(), flags)?;
    let profile = chosen.as_ref().map(|c| c.profile);
    let think_secs = flags
        .think_secs
        .or(profile.and_then(|p| p.think_secs))
        .unwrap_or(baylee_client_core::llmseat::DEFAULT_THINK_SECS);
    if think_secs == 0 {
        return Err("--think-secs is the longest one answer may take: at least 1".into());
    }
    Ok(Some(Plan {
        key_env: profile
            .map_or_else(|| spec.provider.default_key_env(), Profile::key_env)
            .map(str::to_string),
        base_url: profile.and_then(|p| p.base_url.clone()),
        command: profile.and_then(|p| p.command.clone()),
        profile: chosen.map(|c| c.name.to_string()),
        spec,
        settings,
        think_secs,
        note,
    }))
}

/// The profile `named`, or the file's default when none is.
fn pick<'a>(
    file: Option<&'a SeatSettings>,
    paths: &Paths,
    named: Option<&'a str>,
) -> Result<Option<Chosen<'a>>, String> {
    Ok(match (named, file) {
        (Some(name), None) => {
            return Err(format!(
                "--profile {name} names a profile of the settings file, and there is none at \
                 {}",
                paths.named_file()
            ));
        }
        (Some(name), Some(file)) => {
            let Some(profile) = file.profile(name) else {
                return Err(format!(
                    "the settings file {} has no profile «{name}»{}",
                    paths.named_file(),
                    file.known()
                ));
            };
            Some(Chosen {
                name,
                profile,
                named: true,
            })
        }
        (None, Some(file)) => file.default.as_deref().and_then(|name| {
            file.profile(name).map(|profile| Chosen {
                name,
                profile,
                named: false,
            })
        }),
        (None, None) => None,
    })
}

/// How `spec` plays: the build's defaults, then `chosen`'s, then `flags`.
fn tune(spec: &Spec, chosen: Option<&Chosen>, flags: &Overrides) -> Result<Settings, String> {
    let profile = chosen.map(|c| c.profile);
    let mut settings = Settings::new(spec);
    if let Some(profile) = profile {
        if let Some(effort) = profile.effort.as_ref().filter(|_| !flags.default_effort) {
            settings.effort = Some(effort.clone());
        }
        if let Some(answer) = profile.answer {
            settings.answer = answer;
        }
        if let Some(max_tokens) = profile.max_tokens {
            settings.max_tokens = max_tokens;
        }
    }
    if let Some(effort) = &flags.effort {
        if !effort_is_a_word(effort) {
            return Err("an effort is a word such as low, medium or high".into());
        }
        settings.effort = Some(effort.clone());
    }
    if let Some(answer) = flags.answer {
        settings.answer = answer;
    }
    match (spec.provider, settings.answer) {
        (Provider::Anthropic, AnswerMode::Json | AnswerMode::JsonSchema) => {
            return Err(format!(
                "--answer {} is for an OpenAI-compatible endpoint or a CLI; Anthropic's models \
                 answer with tools",
                settings.answer.name().replace('_', "-")
            ));
        }
        (Provider::Cli, AnswerMode::Tools) => {
            return Err(
                "--answer tools is for an API: a CLI plays with no tools, and answers \
                        json-schema or json"
                    .into(),
            );
        }
        _ => {}
    }
    if let Some(max_tokens) = flags.max_tokens {
        if max_tokens == 0 {
            return Err("--max-tokens is the most one reply may take: at least 1".into());
        }
        settings.max_tokens = max_tokens;
    }
    // A profile's price is its model's: a model `--mind` put in its place
    // is counted at this build's price, or at one the command line gives.
    let own_price = profile
        .filter(|p| p.model == spec.model)
        .and_then(|p| p.price)
        .map(|given| Price::per_million(given.input, given.output));
    let spend_usd = flags.spend_usd.or(profile.and_then(|p| p.game_usd));
    let spend_tokens = flags.spend_tokens.or(profile.and_then(|p| p.game_tokens));
    settings
        .budget(flags.price.or(own_price), spend_usd, spend_tokens)
        .map_err(|why| match chosen {
            Some(chosen) if spec.provider != Provider::Cli => format!(
                "{why}; in the settings file, profile «{}» takes a price, or game_tokens",
                chosen.name
            ),
            _ => why,
        })?;
    if let Some(calls) = flags.spend_calls.or(profile.and_then(|p| p.game_calls)) {
        if calls == 0 {
            return Err("--spend-calls is the most calls a game may make: at least 1".into());
        }
        settings.spend_calls = Some(calls);
    }
    Ok(settings)
}

#[cfg(test)]
mod tests;
