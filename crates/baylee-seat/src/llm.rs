//! A language model behind an API, as a mind (`llm-seat.md` §4.4–§4.6, §8).
//!
//! [`ApiMind`] tells each real decision as one message ([`crate::narrator`]),
//! sends it to the model with two tools, `decide` and `concede`, and reads
//! the call back against the question's own options ([`Menu::resolve`]).
//! Two providers: the Anthropic Messages API and any OpenAI-compatible chat
//! endpoint (`DeepSeek`, `OpenAI`, a local server), the latter also without
//! tools, answering one JSON object ([`AnswerMode::Json`]), or one held to
//! the answer's schema ([`AnswerMode::JsonSchema`]).
//!
//! # One conversation per turn
//!
//! A seat's conversation starts with the game and its deck (the cached
//! prefix) and grows by one decision and one answer at a time, appended
//! and never edited, so the provider's cache and the model's own thinking
//! carry from one decision to the next. At each new turn it starts again
//! from the prefix, with the seat's own earlier `say` lines as its notes:
//! the board in every message is whole, so nothing else is lost. A turn
//! whose conversation outgrows [`Settings::conversation_tokens`] starts
//! again the same way at its next question.
//!
//! # Answers the model does not need to give
//!
//! An option that taps lands and then casts is a plan: the taps and the
//! cast go out as the table asks for them, without another call, and a plan
//! the table interrupts stops and says so in the next message. Targets named
//! with `then` answer the target question the cast asks, when they are
//! still legal then.
//!
//! # When the model does not answer
//!
//! An answer the menu cannot read is sent back once with the reason, if the
//! time allows; after that, and for a model that refuses, the house answers
//! ([`MindError::Declined`]). A network or provider failure is
//! [`MindError::Unavailable`], and a run of them takes the mind off the
//! table until [`Mind::ready`] says the endpoint answers again. A request
//! the provider turns down as such (400, 413) would be turned down again
//! with the same history, so the next question starts a new conversation,
//! as a new turn does. Past the
//! game's budget in tokens or dollars (§8) every question is declined and
//! the house finishes the game. A dollar budget needs the model's price, so
//! a model with none sits down only with a token budget stated as its
//! limit ([`Settings::budget`]).
//!
//! # The key
//!
//! Read from the environment only ([`credentials`]), held in a [`Secret`],
//! put in one request header, and blanked from every text the mind writes
//! ([`scrub`]): errors, transcripts, the terminal.

mod anthropic;
mod openai;
pub mod prompt;
pub(crate) mod seatstate;
mod secret;
#[cfg(test)]
mod tests;

pub(crate) use openai::json_object;
pub use secret::{Secret, scrub};

use crate::config::Plan;
use crate::mind::{Answer, Disclosure, Mind, MindError, Readiness, Request, Thinking};
use crate::narrator::{self, Decision, Menu, Narrator};
use baylee_client_core::llmseat::{
    address_fault, is_loopback, model_fault, worst_tokens, worst_usd,
};
use baylee_engine::choice::PlayerAction;
use seatstate::Seat;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// The model's price and the build's defaults live beside the settings
/// file's types, so the client's panel shows what the bridge plays with.
pub use baylee_client_core::llmseat::{
    AnswerMode, CliTool, DEFAULT_ANTHROPIC_MODEL, DEFAULT_CLI_CALLS, DEFAULT_CLI_SPEND_TOKENS,
    DEFAULT_SPEND_TOKENS, DEFAULT_SPEND_USD, Price, Provider, cli_model, price,
};

/// The least time worth a second call after an unreadable answer.
pub(crate) const RETRY_FLOOR: Duration = Duration::from_secs(5);

/// How much sooner than the bridge's deadline a call gives up, so the
/// answer, if any, still reaches the seat.
pub(crate) const MARGIN: Duration = Duration::from_secs(1);

/// The most earlier notes a new turn's conversation carries.
pub(crate) const SAYS: usize = 8;

/// A model, as `--mind` names it: `anthropic`, `anthropic:<model>`,
/// `openai:<model>`, or an agent CLI, `cli:<tool>[:<model>]`, whose model
/// is then `<tool>[:<model>]` ([`cli_model`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The API.
    pub provider: Provider,
    /// The model's id.
    pub model: String,
}

impl Spec {
    /// Reads `anthropic[:<model>]`, `openai:<model>` or
    /// `cli:<tool>[:<model>]`; `None` for anything else (the other minds).
    ///
    /// # Errors
    /// For `openai` without a model, `cli` without a tool or with one this
    /// build does not speak, or a model id with characters no provider
    /// uses.
    pub fn parse(text: &str) -> Option<Result<Self, String>> {
        let (provider, model) = match text.split_once(':') {
            Some((provider, model)) => (provider, Some(model)),
            None => (text, None),
        };
        let provider = match provider {
            "anthropic" => Provider::Anthropic,
            "openai" => Provider::OpenAi,
            "cli" => Provider::Cli,
            _ => return None,
        };
        let model = match (provider, model) {
            (Provider::Anthropic, None) => DEFAULT_ANTHROPIC_MODEL.to_string(),
            (Provider::OpenAi, None) => {
                return Some(Err("name the model: openai:<model>".into()));
            }
            (Provider::Cli, None) => {
                return Some(Err(
                    "name the CLI: cli:<tool>[:<model>], such as cli:claude:opus".into(),
                ));
            }
            (_, Some(model)) => model.trim().to_string(),
        };
        let fault = match provider {
            Provider::Cli => cli_model(&model).err(),
            Provider::Anthropic | Provider::OpenAi => model_fault(&model),
        };
        if let Some(why) = fault {
            return Some(Err(why));
        }
        Some(Ok(Self { provider, model }))
    }

    /// The name a chair it plays sits under, after `LLM-`: the model's id
    /// without `claude-`, in the characters a seat name allows, at most
    /// twelve. A CLI's is its tool's name and its model's (`claude-opus`)
    /// where both fit, else its model's alone, else the tool's.
    #[must_use]
    pub fn tag(&self) -> String {
        if self.provider == Provider::Cli
            && let Ok((tool, own)) = cli_model(&self.model)
        {
            let Some(own) = own else {
                return tag_of(tool.name());
            };
            let own = tag_of(own);
            let both = format!("{}-{own}", tool.name());
            return if both.len() <= 12 { both } else { own };
        }
        tag_of(&self.model)
    }
}

/// A model id as a chair's name ([`Spec::tag`]).
fn tag_of(model: &str) -> String {
    let bare = model.strip_prefix("claude-").unwrap_or(model);
    let bare = bare.rsplit('/').next().unwrap_or(bare);
    let mut tag: String = bare
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if tag.len() > 12 {
        // Cut at a word where one is near: `deepseek`, not `deepseek-cha`.
        tag.truncate(12);
        if let Some(at) = tag.rfind(['-', '_']).filter(|at| *at >= 3) {
            tag.truncate(at);
        }
    }
    while tag.ends_with(['-', '_']) {
        tag.pop();
    }
    if tag.len() < 3 {
        tag = "model".into();
    }
    tag
}

/// How a language model plays, behind an API ([`ApiMind`]) or an agent
/// CLI ([`crate::cli::CliMind`]).
#[derive(Clone, Debug)]
pub struct Settings {
    /// The API.
    pub provider: Provider,
    /// The model's id.
    pub model: String,
    /// How hard the model thinks (`low`, `medium`, `high`, …): Anthropic's
    /// `output_config.effort`, an OpenAI-compatible `reasoning_effort`.
    pub effort: Option<String>,
    /// How it answers.
    pub answer: AnswerMode,
    /// The most tokens one reply may take, thinking included.
    pub max_tokens: u32,
    /// What the model costs: this build's price for it ([`price`]) or one
    /// given ([`Settings::budget`]); `None` when neither is known.
    pub price: Option<Price>,
    /// The game's budget in US dollars. Only with a price: without one no
    /// dollar is counted, and the token budget is the limit.
    pub spend_usd: Option<f64>,
    /// The game's budget in tokens, input and output together.
    pub spend_tokens: u64,
    /// The most calls the game may make, where that is a limit: a CLI's
    /// subscription has no price, and its calls are what it counts.
    pub spend_calls: Option<u64>,
    /// How long a conversation may grow, in estimated tokens, before the
    /// next question starts a new one with the notes carried over: a turn's
    /// through an API, a whole game's through a CLI ([`crate::cli`]), which
    /// keeps one across turns. Neither outgrows the model's context, nor
    /// has every call send more than this again.
    pub conversation_tokens: usize,
    /// A directory for the mind's own transcript: every message it sent and
    /// every reply, one JSON line each.
    pub transcripts: Option<PathBuf>,
    /// Whether the game's budgets are a hard limit, as they are under a
    /// reservation in the spend book ([`crate::spend`]): every call is held
    /// at the most it can cost ([`Settings::worst`]) before it is sent, and
    /// not sent when that could pass a budget, so a game never spends more
    /// than it reserved. Without it, as with no settings file, the budget
    /// is checked before each question and one call may pass it.
    pub hard_limit: bool,
}

impl Settings {
    /// The defaults for `spec`: medium effort on Anthropic, the provider's
    /// own default elsewhere; tools; five million tokens a game, and five
    /// dollars where this build knows the model's price. A CLI answers with
    /// one JSON object held to the answer's schema, and plays on a
    /// subscription, which has no price: twenty million tokens a game
    /// ([`DEFAULT_CLI_SPEND_TOKENS`]), its whole context read again at
    /// every decision counted among them, and 500 calls
    /// ([`DEFAULT_CLI_CALLS`]).
    #[must_use]
    pub fn new(spec: &Spec) -> Self {
        let cli = spec.provider == Provider::Cli;
        let price = if cli { None } else { price(&spec.model) };
        Self {
            provider: spec.provider,
            model: spec.model.clone(),
            effort: spec.provider.default_effort().map(str::to_string),
            answer: spec.provider.default_answer(),
            max_tokens: spec.provider.default_max_tokens(),
            price,
            spend_usd: price.map(|_| DEFAULT_SPEND_USD),
            spend_tokens: if cli {
                DEFAULT_CLI_SPEND_TOKENS
            } else {
                DEFAULT_SPEND_TOKENS
            },
            spend_calls: cli.then_some(DEFAULT_CLI_CALLS),
            conversation_tokens: 100_000,
            transcripts: None,
            hard_limit: false,
        }
    }

    /// The most one call with a request of `bytes` can use: every byte as
    /// an input token at the dearest rate, the provider's own allowance,
    /// and a whole reply of [`Self::max_tokens`].
    #[must_use]
    pub fn worst(&self, bytes: usize) -> Worst {
        let bytes = bytes as u64;
        Worst {
            tokens: worst_tokens(bytes, self.max_tokens),
            usd: self
                .price
                .map(|price| worst_usd(bytes, self.max_tokens, price)),
        }
    }

    /// The price and the budget a command line states: `price`, where one
    /// is given, over this build's; the dollar and token budgets where
    /// given, else the defaults.
    ///
    /// # Errors
    /// A dollar budget cannot be held without a price, so a model that has
    /// none sits down only with a token budget stated as its limit, and
    /// never with a dollar budget. A CLI's subscription has no price, so it
    /// takes neither a price nor a dollar budget, and its token budget is
    /// its own default where none is given.
    pub fn budget(
        &mut self,
        price: Option<Price>,
        spend_usd: Option<f64>,
        spend_tokens: Option<u64>,
    ) -> Result<(), String> {
        if self.provider == Provider::Cli {
            if price.is_some() || spend_usd.is_some() {
                return Err(
                    "a CLI plays on a subscription, which has no price: a price and a \
                            dollar budget are for an API, and a CLI's limits are its tokens \
                            and its calls (--spend-tokens, --spend-calls)"
                        .into(),
                );
            }
            self.price = None;
            self.spend_usd = None;
            if let Some(tokens) = spend_tokens {
                self.spend_tokens = tokens;
            }
            return Ok(());
        }
        if price.is_some() {
            self.price = price;
        }
        if self.price.is_some() {
            self.spend_usd = Some(spend_usd.unwrap_or(DEFAULT_SPEND_USD));
            self.spend_tokens = spend_tokens.unwrap_or(DEFAULT_SPEND_TOKENS);
            return Ok(());
        }
        let model = &self.model;
        if spend_usd.is_some() {
            return Err(format!(
                "this build has no price for «{model}», so --spend-usd cannot be held: give its \
                 price with --price-in and --price-out (US dollars per million tokens)"
            ));
        }
        let Some(tokens) = spend_tokens else {
            return Err(format!(
                "this build has no price for «{model}», so no dollar budget can be held: give its \
                 price with --price-in and --price-out (US dollars per million tokens), or make \
                 a token budget the limit with --spend-tokens N"
            ));
        };
        self.spend_usd = None;
        self.spend_tokens = tokens;
        Ok(())
    }
}

/// The two APIs an [`ApiMind`] speaks; a CLI ([`crate::cli::CliMind`]) is
/// none of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Api {
    Anthropic,
    OpenAi,
}

impl Api {
    const fn of(provider: Provider) -> Option<Self> {
        match provider {
            Provider::Anthropic => Some(Self::Anthropic),
            Provider::OpenAi => Some(Self::OpenAi),
            Provider::Cli => None,
        }
    }
}

/// Where the API is and the key it takes.
#[derive(Clone, Debug)]
pub struct Credentials {
    api: Api,
    key: Option<Secret>,
    base: String,
}

/// Why a CLI has no credentials.
const NOT_AN_API: &str = "a CLI is a program on this machine, not an API: it reads no key and \
                          has no address";

/// The key and address for `provider`, read by `env` (the process
/// environment in the binary, a table in tests): `ANTHROPIC_API_KEY` and
/// `ANTHROPIC_BASE_URL`, or `BAYLEE_LLM_API_KEY` and `BAYLEE_LLM_BASE_URL`.
///
/// # Errors
/// Without a key (an OpenAI-compatible server on this machine may go
/// without), or for an address that would carry the key in the clear.
pub fn credentials(
    provider: Provider,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Credentials, String> {
    let key_env = provider.default_key_env().ok_or(NOT_AN_API)?;
    credentials_at(provider, key_env, None, env)
}

/// The key in the environment variable `key_env`, for the address `base`
/// where a profile names one, else the provider's variable
/// ([`Provider::base_env`]), else the provider's own address. A key named
/// beside an address goes to that address and no other, so a profile's
/// `base_url` wins over the variable.
///
/// # Errors
/// As [`credentials`], and for a CLI, which is no API.
pub fn credentials_at(
    provider: Provider,
    key_env: &str,
    base: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Credentials, String> {
    let (Some(api), Some(base_env), Some(default_base)) = (
        Api::of(provider),
        provider.base_env(),
        provider.default_base(),
    ) else {
        return Err(NOT_AN_API.into());
    };
    let key = env(key_env).as_deref().and_then(Secret::new);
    let tidy = |b: &str| b.trim().trim_end_matches('/').to_string();
    let (base, named) = match base {
        Some(base) => (tidy(base), "base_url"),
        None => (
            env(base_env)
                .map(|b| tidy(&b))
                .filter(|b| !b.is_empty())
                .unwrap_or_else(|| default_base.to_string()),
            base_env,
        ),
    };
    if let Some(why) = address_fault(&base, named) {
        return Err(why);
    }
    if key.is_none() && !(provider == Provider::OpenAi && is_loopback(&base)) {
        return Err(format!(
            "set {key_env} in the environment (never on the command line)"
        ));
    }
    Ok(Credentials { api, key, base })
}

/// What a plan's mind is reached through, checked before the game reserves
/// anything in the spend book: an API's address and key, or a CLI's
/// program and the variables it is given.
#[derive(Debug)]
pub enum Access {
    /// An API.
    Api(Credentials),
    /// An agent CLI.
    Cli(crate::cli::Launch),
}

/// A language model's mind, built from a plan ([`build`]).
pub struct Built {
    /// The mind.
    pub mind: Arc<dyn Mind>,
    /// What it spends, shared: it keeps counting while the mind plays.
    pub tally: Arc<Mutex<Tally>>,
    /// The name its chair sits under, after `LLM-` ([`Spec::tag`]).
    pub label: String,
}

/// What `plan`'s mind is reached through, read by `env` (the process
/// environment in the binary, a table in tests): for an API its key and
/// address ([`credentials_at`]), for a CLI its program and the variables
/// it is given ([`crate::cli::Launch::new`]).
///
/// # Errors
/// A sentence: no key, an address that would carry it in the clear, a CLI
/// whose program is not there or whose environment holds a key.
pub fn check(plan: &Plan, env: &dyn Fn(&str) -> Option<String>) -> Result<Access, String> {
    let provider = plan.settings.provider;
    if provider == Provider::Cli {
        return crate::cli::Launch::new(&plan.settings, plan.command.as_deref(), env)
            .map(Access::Cli);
    }
    let key_env = plan
        .key_env
        .as_deref()
        .or(provider.default_key_env())
        .ok_or(NOT_AN_API)?;
    credentials_at(provider, key_env, plan.base_url.as_deref(), env).map(Access::Api)
}

/// `plan`'s mind: [`ApiMind`] for an API, [`crate::cli::CliMind`] for a
/// CLI, as [`check`] finds it.
///
/// # Errors
/// As [`check`].
pub fn build(plan: &Plan, env: &dyn Fn(&str) -> Option<String>) -> Result<Built, String> {
    Ok(check(plan, env)?.build(plan))
}

impl Access {
    /// The mind that plays `plan` through what was checked; the plan's
    /// settings as they are now, a reservation's hard limit included.
    #[must_use]
    pub fn build(self, plan: &Plan) -> Built {
        let label = plan.spec.tag();
        match self {
            Self::Api(credentials) => {
                let mind = ApiMind::new(plan.settings.clone(), credentials);
                let tally = mind.tally();
                Built {
                    mind: Arc::new(mind),
                    tally,
                    label,
                }
            }
            Self::Cli(launch) => {
                let mind = crate::cli::CliMind::new(
                    plan.settings.clone(),
                    launch,
                    crate::cli::Limits::default(),
                );
                let tally = mind.tally();
                Built {
                    mind: Arc::new(mind),
                    tally,
                    label,
                }
            }
        }
    }
}

/// Tokens as a provider counted them for one reply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Usage {
    /// Input not read from the cache.
    pub input: u64,
    /// Output, thinking included.
    pub output: u64,
    /// Input written to the cache.
    pub cache_write: u64,
    /// Input read from the cache.
    pub cache_read: u64,
}

impl Usage {
    /// Every token, in and out.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.input + self.output + self.cache_write + self.cache_read
    }

    /// What it cost at `price`.
    #[must_use]
    pub fn cost(&self, price: Price) -> f64 {
        #[allow(clippy::cast_precision_loss)] // token counts stay far below 2^52
        let at = |n: u64, per: f64| n as f64 * per / 1_000_000.0;
        at(self.input, price.input)
            + at(self.output, price.output)
            + at(self.cache_write, price.cache_write)
            + at(self.cache_read, price.cache_read)
    }
}

/// What a mind has spent, over every game it played.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Tally {
    /// Calls made, the ones that came back too late included.
    pub calls: u64,
    /// Calls that failed: the network, the provider, an unreadable body.
    pub failed: u64,
    /// Tokens.
    pub usage: Usage,
    /// Dollars, where the price is known.
    pub usd: Option<f64>,
    /// Whether a game's budget ran out.
    pub spent: bool,
    /// The most the calls still out may cost, held until they come back.
    pub held: Worst,
    /// The most the failed calls whose bill nobody knows may have cost: a
    /// call that timed out, or whose reply could not be read, may still
    /// have been billed in full.
    pub unsure: Worst,
    /// Calls sent and not yet back.
    pub out: u64,
    /// The most calls a game may make, where that is a limit
    /// ([`Settings::spend_calls`]), for the summary.
    pub calls_cap: Option<u64>,
    /// The conversations a CLI's processes held ([`crate::cli`]): one a
    /// seat, kept across turns, and another each time one outgrew its
    /// size or was lost.
    pub sessions: u64,
    /// Of them, the ones begun again because the one before was lost: its
    /// process died, hung or sat idle, and the prefix was sent again.
    pub restarts: u64,
}

/// The most calls can cost: tokens, and dollars where the price is known.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Worst {
    /// Tokens, in and out.
    pub tokens: u64,
    /// US dollars.
    pub usd: Option<f64>,
}

impl Worst {
    fn add(&mut self, other: Self) {
        self.tokens = self.tokens.saturating_add(other.tokens);
        if let Some(usd) = other.usd {
            *self.usd.get_or_insert(0.0) += usd;
        }
    }

    fn sub(&mut self, other: Self) {
        self.tokens = self.tokens.saturating_sub(other.tokens);
        if let (Some(held), Some(usd)) = (&mut self.usd, other.usd) {
            *held = (*held - usd).max(0.0);
        }
        // Every call holds some tokens, so none held is no call out, and
        // what a float subtraction leaves over is not a dollar.
        if self.tokens == 0 {
            self.usd = None;
        }
    }
}

impl Tally {
    /// The most the game may have cost in tokens: what the provider
    /// counted, and at their worst the calls still out and the calls whose
    /// bill is unknown. What the spend book settles with.
    #[must_use]
    pub const fn spend_tokens(&self) -> u64 {
        self.usage
            .total()
            .saturating_add(self.held.tokens)
            .saturating_add(self.unsure.tokens)
    }

    /// The same in dollars, where the price is known.
    #[must_use]
    pub fn spend_usd(&self) -> Option<f64> {
        let parts = [self.usd, self.held.usd, self.unsure.usd];
        parts
            .iter()
            .any(Option::is_some)
            .then(|| parts.iter().flatten().sum())
    }

    /// Why the game's budget under `settings` is spent, if it is; then the
    /// tally remembers that it is. Under a hard limit the calls still out
    /// and those whose bill is unknown count at their worst; a call cap
    /// counts the calls made and those out.
    pub(crate) fn spent_under(&mut self, settings: &Settings) -> Option<String> {
        let (tokens, usd) = if settings.hard_limit {
            (self.spend_tokens(), self.spend_usd().unwrap_or(0.0))
        } else {
            (self.usage.total(), self.usd.unwrap_or(0.0))
        };
        let why = if tokens >= settings.spend_tokens {
            format!(
                "the game's budget of {} tokens is spent",
                settings.spend_tokens
            )
        } else if let Some(budget) = settings.spend_usd
            && usd >= budget
        {
            format!("the game's budget of ${budget:.2} is spent (${usd:.2})")
        } else if let Some(cap) = settings.spend_calls
            && self.calls.saturating_add(self.out) >= cap
        {
            format!("the game's budget of {cap} calls is spent")
        } else {
            return None;
        };
        self.spent = true;
        Some(why)
    }

    /// Holds `worst` for a call about to be sent, unless it could pass the
    /// game's hard limit ([`Settings::hard_limit`]); then the sentence why.
    pub(crate) fn hold(&mut self, worst: Worst, settings: &Settings) -> Result<(), String> {
        if settings.hard_limit {
            if self.spend_tokens().saturating_add(worst.tokens) > settings.spend_tokens {
                return Err(format!(
                    "the game's budget of {} tokens cannot hold another call, which may take \
                     up to {}",
                    settings.spend_tokens, worst.tokens
                ));
            }
            if let (Some(budget), Some(usd)) = (settings.spend_usd, worst.usd)
                && self.spend_usd().unwrap_or(0.0) + usd > budget
            {
                return Err(format!(
                    "the game's budget of ${budget:.2} cannot hold another call, which may cost \
                     up to ${usd:.2}"
                ));
            }
        }
        if let Some(cap) = settings.spend_calls
            && self.calls.saturating_add(self.out) >= cap
        {
            return Err(format!("the game's budget of {cap} calls is spent"));
        }
        self.held.add(worst);
        self.out += 1;
        Ok(())
    }

    /// Takes back what [`Self::hold`] held for a call that came back, and
    /// counts how it came back: its tokens as the provider counted them, or
    /// its worst when the bill is unknown.
    pub(crate) fn back(&mut self, worst: Worst, billed: Result<Usage, bool>, price: Option<Price>) {
        self.held.sub(worst);
        self.out = self.out.saturating_sub(1);
        self.calls += 1;
        match billed {
            Ok(usage) => self.add(usage, price),
            Err(unknown) => {
                self.failed += 1;
                if unknown {
                    self.unsure.add(worst);
                }
            }
        }
    }

    /// Takes back what [`Self::hold`] held for a call that came back
    /// answered without saying what it cost: it counts at its worst, though
    /// it did not fail.
    pub(crate) fn back_at_worst(&mut self, worst: Worst) {
        self.held.sub(worst);
        self.out = self.out.saturating_sub(1);
        self.calls += 1;
        self.unsure.add(worst);
    }

    fn add(&mut self, usage: Usage, price: Option<Price>) {
        self.usage.input += usage.input;
        self.usage.output += usage.output;
        self.usage.cache_write += usage.cache_write;
        self.usage.cache_read += usage.cache_read;
        if let Some(price) = price {
            *self.usd.get_or_insert(0.0) += usage.cost(price);
        }
    }
}

/// One reply, as both providers read.
#[derive(Clone, Debug)]
pub struct Reply {
    /// The reply as the conversation replays it.
    assistant: Value,
    /// The tool calls, in order.
    calls: Vec<Call>,
    /// What the model wrote outside its calls.
    text: String,
    /// Its reasoning, where the provider shows it.
    reasoning: Option<String>,
    /// Its tokens.
    usage: Usage,
    /// Why it stopped.
    stop: Stop,
}

/// One tool call.
#[derive(Clone, Debug)]
struct Call {
    id: String,
    name: String,
    input: Value,
}

/// Why a reply stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Stop {
    Done,
    Refusal,
    MaxTokens,
    Other(String),
}

/// What the seat says back for a tool call: done, or refused and why.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ToolResult {
    id: String,
    content: String,
    is_error: bool,
}

/// What one seat's conversation holds between decisions: what every
/// language-model seat keeps ([`Seat`]), and the conversation as the API
/// takes it.
struct SeatState {
    seat: Seat,
    /// The conversation, as the provider takes it.
    messages: Vec<Value>,
    /// What the seat owes the model's last tool calls.
    results: Vec<ToolResult>,
}

/// Each seat's conversation, by game and seat.
type Seats = Mutex<BTreeMap<(String, u8), Arc<Mutex<SeatState>>>>;

/// A language model behind an API.
pub struct ApiMind {
    settings: Settings,
    credentials: Credentials,
    agent: ureq::Agent,
    seats: Seats,
    tally: Arc<Mutex<Tally>>,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ApiMind {
    /// A mind that plays with `settings` against the API `credentials`
    /// name.
    #[must_use]
    pub fn new(settings: Settings, credentials: Credentials) -> Self {
        // No redirect is followed: ureq drops only `Authorization` and
        // `Cookie` when it follows one, so Anthropic's `x-api-key` would go
        // wherever a `Location` pointed, plain http included. A 3xx is an
        // error ([`call`]). And nothing leaves this machine but over TLS, as
        // [`credentials`] requires of the address.
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .https_only(!is_loopback(&credentials.base))
            .timeout_global(Some(Duration::from_secs(600)))
            .build()
            .new_agent();
        let tally = Tally {
            calls_cap: settings.spend_calls,
            ..Tally::default()
        };
        Self {
            settings,
            credentials,
            agent,
            seats: Mutex::new(BTreeMap::new()),
            tally: Arc::new(Mutex::new(tally)),
        }
    }

    /// What the mind has spent so far, shared: it keeps counting while the
    /// mind plays.
    #[must_use]
    pub fn tally(&self) -> Arc<Mutex<Tally>> {
        Arc::clone(&self.tally)
    }

    /// The settings it plays with.
    #[must_use]
    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    fn seat(&self, request: &Request) -> Arc<Mutex<SeatState>> {
        let context = &request.context;
        let key = (context.game_id.clone(), context.seat.get());
        let mut seats = lock(&self.seats);
        Arc::clone(seats.entry(key).or_insert_with(|| {
            Arc::new(Mutex::new(SeatState {
                seat: Seat::new(context, self.settings.transcripts.as_deref()),
                messages: Vec::new(),
                results: Vec::new(),
            }))
        }))
    }

    /// Why the game's budget is spent, if it is.
    fn spent(&self) -> Option<String> {
        lock(&self.tally).spent_under(&self.settings)
    }

    /// The request's headers, the key among them.
    fn headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![("content-type", "application/json".to_string())];
        match self.credentials.api {
            Api::Anthropic => {
                headers.push(("anthropic-version", anthropic::VERSION.to_string()));
                if let Some(key) = &self.credentials.key {
                    headers.push(("x-api-key", key.expose().to_string()));
                }
            }
            Api::OpenAi => {
                if let Some(key) = &self.credentials.key {
                    headers.push(("authorization", format!("Bearer {}", key.expose())));
                }
            }
        }
        headers
    }

    /// Posts `body` and reads the reply, counting its tokens even when the
    /// seat stopped waiting (`alive` false): the provider bills it anyway,
    /// and the next message says the answer came late.
    async fn post(
        &self,
        body: Value,
        timeout: Duration,
        alive: Arc<AtomicBool>,
        seat: Arc<Mutex<SeatState>>,
        question: u64,
    ) -> Result<Reply, MindError> {
        let url = match self.credentials.api {
            Api::Anthropic => anthropic::url(&self.credentials.base),
            Api::OpenAi => openai::url(&self.credentials.base),
        };
        let headers = self.headers();
        let agent = self.agent.clone();
        let tally = Arc::clone(&self.tally);
        let price = self.settings.price;
        let api = self.credentials.api;
        let mode = self.settings.answer;
        let key = self.credentials.key.clone();
        let bytes = serde_json::to_vec(&body)
            .map_err(|e| MindError::Unavailable(format!("the request is not JSON: {e}")))?;
        // Held until the call comes back, whenever that is: the blocking
        // task below runs to its end even when the seat stops waiting.
        let worst = self.settings.worst(bytes.len());
        {
            let mut tally = lock(&self.tally);
            if let Err(why) = tally.hold(worst, &self.settings) {
                tally.spent = true;
                return Err(MindError::Declined(why));
            }
        }
        let result = tokio::task::spawn_blocking(move || {
            let result = call(&agent, &url, &headers, &bytes, timeout, api, mode);
            let billed = match &result {
                Ok(reply) => Ok(reply.usage),
                Err(failed) => Err(failed.unknown_bill),
            };
            lock(&tally).back(worst, billed, price);
            if !alive.load(Ordering::SeqCst) && result.is_ok() {
                lock(&seat).seat.late = Some(question);
            }
            // A conversation the provider turned down would be turned
            // down at every question until the turn ends and take the
            // mind off the table: the next question starts a new one, with
            // the notes carried over as at a new turn.
            if result.as_ref().is_err_and(|failed| failed.turned_down) {
                let mut seat = lock(&seat);
                seat.messages.clear();
                seat.results.clear();
            }
            result.map_err(|failed| scrub(&failed.why, key.as_ref()))
        })
        .await
        .map_err(|e| MindError::Unavailable(format!("the call did not finish: {e}")))?;
        result.map_err(MindError::Unavailable)
    }

    /// Answers `request`: from the plan or the hint where they fit, else by
    /// asking the model.
    async fn think(&self, request: Request) -> Result<Answer, MindError> {
        let started = Instant::now();
        let seat = self.seat(&request);
        let alive = Arc::new(AtomicBool::new(true));
        let _guard = Alive(Arc::clone(&alive));
        let prepared = match self.start(&seat, &request) {
            Ok(prepared) => prepared,
            Err(done) => return done,
        };
        let mut messages = prepared.messages;
        let mut tries = 0;
        loop {
            tries += 1;
            let left = request
                .budget
                .saturating_sub(started.elapsed())
                .saturating_sub(MARGIN);
            if left.is_zero() {
                return Err(MindError::Declined("no time left to ask the model".into()));
            }
            let body = match self.credentials.api {
                Api::Anthropic => anthropic::body(&self.settings, &messages),
                Api::OpenAi => openai::body(&self.settings, &messages),
            };
            let sent = Instant::now();
            let reply = self
                .post(
                    body,
                    left,
                    Arc::clone(&alive),
                    Arc::clone(&seat),
                    request.question,
                )
                .await;
            let reply = match reply {
                Ok(reply) => reply,
                Err(error) => {
                    lock(&seat).record(&request, &prepared.wake.text, None, Some(&error));
                    return Err(error);
                }
            };
            let read = read(&reply, &prepared.wake.menu, self.settings.answer);
            let mut state = lock(&seat);
            state.record(&request, &prepared.wake.text, Some(&reply), None);
            match read {
                Ok(read) => {
                    if state.seat.asked != prepared.asked {
                        return Err(MindError::Declined("a newer question replaced it".into()));
                    }
                    messages.push(reply.assistant.clone());
                    let answer = state.commit(
                        &request,
                        messages,
                        prepared.narrator,
                        &reply,
                        read,
                        started.elapsed(),
                    );
                    let mut answer = answer;
                    answer.model_time = sent.elapsed();
                    return Ok(answer);
                }
                Err(why) => {
                    let again = request
                        .budget
                        .saturating_sub(started.elapsed())
                        .saturating_sub(MARGIN);
                    if reply.stop == Stop::Refusal {
                        return Err(MindError::Declined("the model refused to answer".into()));
                    }
                    if tries >= 2 || again < RETRY_FLOOR {
                        return Err(MindError::Declined(format!(
                            "the model's answer could not be read: {why}"
                        )));
                    }
                    drop(state);
                    ask_again(
                        self.credentials.api,
                        &mut messages,
                        &reply,
                        &why,
                        request.question,
                    );
                }
            }
        }
    }

    /// What is done before the model is asked: the log heard, a refusal or
    /// a late answer noted, the plan or the hint followed, the budget
    /// checked, the message told. `Err` is the answer when no call is made.
    #[allow(clippy::result_large_err)]
    fn start(
        &self,
        seat: &Mutex<SeatState>,
        request: &Request,
    ) -> Result<Prepared, Result<Answer, MindError>> {
        let mut state = lock(seat);
        let SeatState { seat, results, .. } = &mut *state;
        // With tools, the refusal of the model's answer is that call's
        // result.
        let tools = self.settings.answer == AnswerMode::Tools;
        let carried = seat.begin(request, |said| {
            let Some(result) = results.first_mut().filter(|_| tools) else {
                return false;
            };
            said.clone_into(&mut result.content);
            result.is_error = true;
            true
        });
        if let Some(answer) = carried {
            return Err(Ok(answer));
        }
        if let Some(why) = self.spent() {
            return Err(Err(MindError::Declined(why)));
        }
        state.seat.asked += 1;
        Ok(state.prepare(request, &self.settings, self.credentials.api))
    }

    /// Whether the endpoint answers: the model's entry, or the model list.
    async fn probe(&self) -> bool {
        let url = match self.credentials.api {
            Api::Anthropic => anthropic::model_url(&self.credentials.base, &self.settings.model),
            Api::OpenAi => openai::models_url(&self.credentials.base),
        };
        let headers = self.headers();
        let agent = self.agent.clone();
        tokio::task::spawn_blocking(move || {
            let mut request = agent.get(&url);
            for (name, value) in &headers {
                if *name != "content-type" {
                    request = request.header(*name, value);
                }
            }
            request
                .config()
                .timeout_global(Some(Duration::from_secs(10)))
                .build()
                .call()
                .is_ok_and(|answer| answer.status().is_success())
        })
        .await
        .unwrap_or(false)
    }
}

/// Sends an unreadable reply back with the reason, as the conversation's
/// next turn: an error result for each of its calls, and the instruction.
fn ask_again(api: Api, messages: &mut Vec<Value>, reply: &Reply, why: &str, question: u64) {
    messages.push(reply.assistant.clone());
    let results: Vec<ToolResult> = reply
        .calls
        .iter()
        .filter(|c| !c.id.is_empty())
        .map(|c| ToolResult {
            id: c.id.clone(),
            content: why.to_string(),
            is_error: true,
        })
        .collect();
    let text = format!("That answer could not be taken: {why}. Answer q{question} again.");
    match api {
        Api::Anthropic => anthropic::nudge(messages, &results, &text),
        Api::OpenAi => openai::user(messages, &results, None, &text),
    }
}

/// Marks a decision abandoned when the bridge drops its future.
struct Alive(Arc<AtomicBool>);

impl Drop for Alive {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// A call that failed.
struct Failed {
    /// Why, in words (not yet scrubbed).
    why: String,
    /// The provider turned the request itself down (400, 413): the same
    /// conversation would be turned down again.
    turned_down: bool,
    /// Nobody knows whether the provider billed it: the request may have
    /// reached it (a timeout, a dropped connection) or it answered and the
    /// answer could not be read. Such a call counts at its worst. A call
    /// that never reached a server, or that the provider answered with an
    /// error, is not billed.
    unknown_bill: bool,
}

impl From<String> for Failed {
    /// A failure after the provider answered: billed, as far as anybody
    /// knows.
    fn from(why: String) -> Self {
        Self {
            why,
            turned_down: false,
            unknown_bill: true,
        }
    }
}

impl Failed {
    /// A failure the provider does not bill.
    fn unbilled(why: String) -> Self {
        Self {
            unknown_bill: false,
            ..Self::from(why)
        }
    }
}

/// One blocking call of the request `bytes`: the post, the status, the
/// body.
fn call(
    agent: &ureq::Agent,
    url: &str,
    headers: &[(&'static str, String)],
    bytes: &[u8],
    timeout: Duration,
    api: Api,
    mode: AnswerMode,
) -> Result<Reply, Failed> {
    let mut request = agent.post(url);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    let mut answer = request
        .config()
        .timeout_global(Some(timeout))
        .build()
        .send(bytes)
        .map_err(|e| {
            // These fail before a byte of the request reaches a server.
            let unsent = matches!(
                e,
                ureq::Error::HostNotFound
                    | ureq::Error::ConnectionFailed
                    | ureq::Error::BadUri(_)
                    | ureq::Error::RequireHttpsOnly(_)
                    | ureq::Error::InvalidProxyUrl
                    | ureq::Error::TlsRequired
            );
            let why = match e {
                ureq::Error::Timeout(_) => format!("no reply within {} s", timeout.as_secs()),
                other => format!("the call failed: {other}"),
            };
            if unsent {
                Failed::unbilled(why)
            } else {
                Failed::from(why)
            }
        })?;
    let status = answer.status().as_u16();
    let success = (200..300).contains(&status);
    if answer.status().is_redirection() {
        // Named by its status alone: where it pointed, and the body a proxy
        // may have written about it, are not read.
        return Err(Failed::unbilled(redirected(status)));
    }
    let body: Value = answer
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_json()
        .map_err(|e| {
            let why = format!("{status}: the reply is not JSON ({e})");
            if success {
                Failed::from(why)
            } else {
                Failed::unbilled(why)
            }
        })?;
    if !success {
        let why = match api {
            Api::Anthropic => anthropic::error_message(&body),
            Api::OpenAi => openai::error_message(&body),
        }
        .unwrap_or_else(|| "no reason given".into());
        let why = match openai::response_format_hint(&why, mode) {
            Some(hint) if api == Api::OpenAi && status == 400 => format!("{why} ({hint})"),
            _ => why,
        };
        return Err(Failed {
            turned_down: matches!(status, 400 | 413),
            ..Failed::unbilled(format!("{status}: {why}"))
        });
    }
    Ok(match api {
        Api::Anthropic => anthropic::parse(&body),
        Api::OpenAi => openai::parse(&body, mode),
    }?)
}

/// Why a redirect failed the call, by its status alone.
fn redirected(status: u16) -> String {
    format!("{status}: the endpoint answered with a redirect, and the seat follows none")
}

/// An answer read against its question.
struct Read {
    resolved: narrator::Resolved,
    say: Option<String>,
    /// The call it came from, and the other calls, which are ignored.
    call: Option<String>,
    others: Vec<String>,
}

/// Reads a reply's first `decide` or `concede` call against `menu`.
fn read(reply: &Reply, menu: &Menu, mode: AnswerMode) -> Result<Read, String> {
    let mut chosen = None;
    let mut others = Vec::new();
    for call in &reply.calls {
        let known = call.name == "decide" || call.name == "concede";
        if known && chosen.is_none() {
            chosen = Some(call);
        } else if !call.id.is_empty() {
            others.push(call.id.clone());
        }
    }
    let Some(call) = chosen else {
        return Err(match (mode, &reply.stop) {
            (_, Stop::MaxTokens) => "the reply ran out of tokens before it answered".into(),
            (AnswerMode::Tools, _) => "answer with one call of the decide tool".into(),
            (AnswerMode::Json | AnswerMode::JsonSchema, _) => {
                "answer with one JSON object and nothing else".into()
            }
        });
    };
    let decision = if call.name == "concede" {
        Decision::from_concede(&call.input)
    } else {
        Decision::from_decide(&call.input)?
    };
    let resolved = menu.resolve(&decision)?;
    Ok(Read {
        resolved,
        say: decision.say.filter(|s| !s.is_empty()),
        call: (!call.id.is_empty()).then(|| call.id.clone()),
        others,
    })
}

/// A decision told and ready to send.
struct Prepared {
    wake: narrator::Wake,
    /// The narrator after telling it, kept only if the model answers.
    narrator: Narrator,
    messages: Vec<Value>,
    asked: u64,
}

impl SeatState {
    /// Tells the decision and builds the conversation that asks it.
    #[allow(clippy::result_large_err)]
    fn prepare(&mut self, request: &Request, settings: &Settings, api: Api) -> Prepared {
        let long = || {
            serde_json::to_string(&self.messages).map_or(0, |text| narrator::estimate_tokens(&text))
                > settings.conversation_tokens
        };
        let fresh = self.messages.is_empty() || request.view.turn != self.seat.turn || long();
        let told = self.seat.told(fresh);
        let mut narrator = self.seat.narrator.clone();
        if fresh {
            narrator.forget_cards();
        }
        let stops_summary = self.seat.stops_summary();
        let wake = narrator.wake(request, &told, stops_summary.as_deref());
        let prefix = fresh.then(|| narrator::prefix(&request.context));
        let (mut messages, results) = if fresh {
            let start = match api {
                Api::Anthropic => Vec::new(),
                Api::OpenAi => vec![openai::system(settings.answer)],
            };
            (start, Vec::new())
        } else {
            (self.messages.clone(), self.results.clone())
        };
        match api {
            Api::Anthropic => {
                anthropic::user(&mut messages, &results, prefix.as_deref(), &wake.text);
            }
            Api::OpenAi => {
                openai::user(&mut messages, &results, prefix.as_deref(), &wake.text);
            }
        }
        Prepared {
            wake,
            narrator,
            messages,
            asked: self.seat.asked,
        }
    }

    /// Keeps what the model answered, and turns it into the seat's answer.
    fn commit(
        &mut self,
        request: &Request,
        messages: Vec<Value>,
        narrator: Narrator,
        reply: &Reply,
        read: Read,
        took: Duration,
    ) -> Answer {
        self.messages = messages;
        let label = read.resolved.label.clone();
        self.results = read
            .call
            .iter()
            .map(|id| ToolResult {
                id: id.clone(),
                content: format!("Done: {label}."),
                is_error: false,
            })
            .chain(read.others.iter().map(|id| ToolResult {
                id: id.clone(),
                content: "Ignored: one decision per call.".into(),
                is_error: false,
            }))
            .collect();
        let action = self
            .seat
            .keep(request, narrator, read.resolved, read.say.as_deref());
        let note = json!({
            "chose": label,
            "say": read.say,
            "reasoning": reply.reasoning,
            "tokens": reply.usage,
            "ms": u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        });
        let stops = self.seat.stops.clone();
        let hold = self.seat.hold.clone();
        Answer {
            action,
            model_time: took,
            note: Some(note.to_string()),
            thinking: None,
            stops: stops.map(Box::new),
            hold,
        }
    }

    /// One line of the mind's own transcript: the message, the reply.
    fn record(
        &mut self,
        request: &Request,
        text: &str,
        reply: Option<&Reply>,
        error: Option<&MindError>,
    ) {
        self.seat.transcript.write_value(&json!({
            "question": request.question,
            "turn": request.view.turn,
            "message": text,
            "reply": reply.map(|r| json!({
                "text": r.text,
                "reasoning": r.reasoning,
                "calls": r.calls.iter().map(|c| json!({"name": c.name, "input": c.input})).collect::<Vec<_>>(),
                "usage": r.usage,
                "stop": format!("{:?}", r.stop),
            })),
            "error": error.map(ToString::to_string),
        }));
        self.seat.transcript.flush();
    }
}

impl Mind for ApiMind {
    fn decide(&self, request: Request) -> Thinking<'_> {
        Box::pin(async move {
            let seat = self.seat(&request);
            let answer = self.think(request).await;
            if let Ok(Answer {
                action: PlayerAction::CastSpell { card },
                ..
            }) = &answer
            {
                lock(&seat).seat.casting = Some(*card);
            }
            answer
        })
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::Llm
    }

    fn ready(&self) -> Readiness<'_> {
        Box::pin(self.probe())
    }
}
