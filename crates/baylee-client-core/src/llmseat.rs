//! The language-model seat's settings: which model a seat bridge plays,
//! how, and what it may spend, per game and across games
//! (`docs/llm-seat.md`).
//!
//! One file, [`FILE`] in the player's config directory
//! ([`crate::userdirs`]), JSON as the client's own settings are. It holds
//! named [`Profile`]s, the one played when nothing else is named, and the
//! [`Caps`] that every game counts against, which the spend book
//! ([`ledger`]) holds across games. The bridge (`baylee-seat`) reads it;
//! the client's settings panel writes it.
//!
//! No key is ever in it. A profile names the environment variable its key
//! is read from ([`Profile::key_env`]), and a file with a field named like
//! a key, or a value shaped like one, is refused with a sentence saying
//! where keys go ([`SeatSettings::parse`]). Every refusal is also a
//! [`Fault`] with the field it is about ([`SeatSettings::faults`]), which
//! the settings panel ([`panel`]) shows beside that field.
//!
//! Pure data and arithmetic, for every target. Reading and writing the
//! files is [`store`], [`ledger::Book`] and the panel's [`desk`], on native
//! targets only, the one place in this crate that touches a file. Nothing
//! here reads a clock: the ledger is handed the moment
//! ([`ledger::Moment`]).

#[cfg(not(target_arch = "wasm32"))]
pub mod desk;
pub mod ledger;
pub mod panel;
#[cfg(not(target_arch = "wasm32"))]
pub mod store;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The settings file's name in the player's config directory.
pub const FILE: &str = "llm-seat.json";

/// The model `anthropic` plays when no model is named.
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-5-5";

/// A game's dollar budget when none is given, for a model with a price.
pub const DEFAULT_SPEND_USD: f64 = 5.0;

/// A game's token budget when none is given.
pub const DEFAULT_SPEND_TOKENS: u64 = 5_000_000;

/// The longest one answer may take when nothing says otherwise, in seconds.
pub const DEFAULT_THINK_SECS: u64 = 60;

/// A CLI's game budget in tokens when none is given. Cache reads count as
/// every other token does, and a CLI that keeps its conversation across
/// turns sends all of it again with each decision, which it reports as read
/// from the cache: a conversation of ten decisions counts its first one ten
/// times.
pub const DEFAULT_CLI_SPEND_TOKENS: u64 = 20_000_000;

/// The most calls a CLI's game makes when nothing says otherwise: past it
/// the house finishes the game.
pub const DEFAULT_CLI_CALLS: u64 = 500;

/// Which API a model is behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    /// The Anthropic Messages API.
    #[serde(rename = "anthropic")]
    Anthropic,
    /// An OpenAI-compatible chat endpoint (`DeepSeek`, `OpenAI`, a server on
    /// this machine).
    #[serde(rename = "openai")]
    OpenAi,
    /// An agent CLI on this machine that the player is signed in to (Claude
    /// Code), run as a program that holds a seat's conversation across
    /// turns ([`CliTool`]): a subscription, with no key, no address and no
    /// price.
    #[serde(rename = "cli")]
    Cli,
}

impl Provider {
    /// The name `--mind` and the file spell it with.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Cli => "cli",
        }
    }

    /// The environment variable its key is read from unless a profile names
    /// another; `None` for a CLI, which reads no key.
    #[must_use]
    pub const fn default_key_env(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::OpenAi => Some("BAYLEE_LLM_API_KEY"),
            Self::Cli => None,
        }
    }

    /// The environment variable its address is read from unless a profile
    /// names one; `None` for a CLI, which is a program, not an address.
    #[must_use]
    pub const fn base_env(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_BASE_URL"),
            Self::OpenAi => Some("BAYLEE_LLM_BASE_URL"),
            Self::Cli => None,
        }
    }

    /// Its address when neither a profile nor the environment names one;
    /// `None` for a CLI.
    #[must_use]
    pub const fn default_base(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("https://api.anthropic.com"),
            Self::OpenAi => Some("https://api.openai.com/v1"),
            Self::Cli => None,
        }
    }

    /// The most tokens one reply may take, thinking included, when nothing
    /// says otherwise. No CLI takes such a limit: for one, it is only the
    /// allowance for a reply that a call is held to at its worst.
    #[must_use]
    pub const fn default_max_tokens(self) -> u32 {
        match self {
            Self::Anthropic | Self::Cli => 16_000,
            Self::OpenAi => 8_000,
        }
    }

    /// How hard its models think when nothing says otherwise: medium on
    /// Anthropic, the endpoint's or the CLI's own default elsewhere.
    #[must_use]
    pub const fn default_effort(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("medium"),
            Self::OpenAi | Self::Cli => None,
        }
    }

    /// How it answers when nothing says otherwise: by calling a tool, or for
    /// a CLI, which has none, with one JSON object held to the answer's
    /// schema.
    #[must_use]
    pub const fn default_answer(self) -> AnswerMode {
        match self {
            Self::Anthropic | Self::OpenAi => AnswerMode::Tools,
            Self::Cli => AnswerMode::JsonSchema,
        }
    }
}

/// The model a Gemini CLI (Antigravity) profile plays when none is named.
pub const DEFAULT_AGY_MODEL: &str = "gemini-3.8-flash-high";

/// An agent CLI a `cli` profile plays through, named first in its model
/// (`claude:opus`, `agy:gemini-3.8-flash-high`). Only the tools whose dialect
/// this build speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CliTool {
    /// Claude Code, `claude`.
    Claude,
    /// Gemini CLI (Antigravity), `agy`.
    Agy,
}

impl CliTool {
    /// Every tool this build speaks.
    pub const ALL: [Self; 2] = [Self::Claude, Self::Agy];

    /// Its name, which is also the program's name on `PATH`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Agy => "agy",
        }
    }

    /// The tool called `name`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "claude" => Some(Self::Claude),
            "agy" | "gemini" => Some(Self::Agy),
            _ => None,
        }
    }
}

/// A CLI's model as a profile or `--mind cli:` names it: the tool, and the
/// tool's own model id where one is named (`claude:opus`), else the tool's
/// default (`claude`).
///
/// # Errors
/// A sentence, for a tool this build does not speak or a model id with
/// characters no provider uses.
pub fn cli_model(model: &str) -> Result<(CliTool, Option<&str>), String> {
    let (tool, own) = match model.split_once(':') {
        Some((tool, own)) => (tool, Some(own)),
        None => (model, None),
    };
    let tools: Vec<&str> = CliTool::ALL.iter().map(|t| t.name()).collect();
    let Some(tool) = CliTool::named(tool) else {
        return Err(format!(
            "«{}» is not a CLI this build plays: name the tool first, {} (claude:opus names \
             its model too)",
            blank_key_shapes(model).chars().take(40).collect::<String>(),
            tools.join(", ")
        ));
    };
    if let Some(own) = own
        && let Some(why) = model_fault(own)
    {
        return Err(why);
    }
    Ok((tool, own))
}

/// How the model answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerMode {
    /// By calling the `decide` or `concede` tool.
    Tools,
    /// With one JSON object, for an endpoint without tools: the request
    /// asks for an object (`response_format` `json_object`).
    Json,
    /// With one JSON object held to the answer's schema
    /// (`response_format` `json_schema`), for an endpoint that refuses a
    /// bare `json_object`, as LM Studio does.
    JsonSchema,
}

impl AnswerMode {
    /// Whether the model answers in JSON rather than by calling a tool.
    #[must_use]
    pub const fn is_json(self) -> bool {
        matches!(self, Self::Json | Self::JsonSchema)
    }

    /// The name the file spells it with.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Json => "json",
            Self::JsonSchema => "json_schema",
        }
    }
}

/// A model's price per million tokens, in US dollars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Price {
    /// Input.
    pub input: f64,
    /// Output, thinking included.
    pub output: f64,
    /// Input written to the cache for five minutes.
    pub cache_write: f64,
    /// Input written to the cache for an hour: twice the input price at
    /// Anthropic.
    pub cache_write_hour: f64,
    /// Input read from the cache.
    pub cache_read: f64,
}

impl Price {
    /// A price given as input and output, in US dollars per million
    /// tokens, for a model this build has no price for. The cache is
    /// counted so that the bill errs high: a write at 1.25 times the input
    /// price, the ratio of both rows of [`price`], and a read at the full
    /// input price rather than a guessed discount.
    #[must_use]
    pub fn per_million(input: f64, output: f64) -> Self {
        Self {
            input,
            output,
            cache_write: input * 1.25,
            cache_write_hour: input * 2.0,
            cache_read: input,
        }
    }

    /// The dearest way an input token can be billed: plain, written to the
    /// cache for either time, or read from it.
    #[must_use]
    pub fn dearest_input(&self) -> f64 {
        self.input
            .max(self.cache_write)
            .max(self.cache_write_hour)
            .max(self.cache_read)
    }
}

/// Sonnet's price.
const SONNET: Price = Price {
    input: 2.0,
    output: 10.0,
    cache_write: 2.5,
    cache_write_hour: 4.0,
    cache_read: 0.2,
};

/// The models this build knows a price for, with their provider: the one
/// table [`price`] reads, and the models a settings panel suggests. A
/// snapshot: the provider's price list is the authority.
pub const PRICED: &[(Provider, &str, Price)] = &[
    (Provider::Anthropic, "claude-sonnet-5-5", SONNET),
    (Provider::Anthropic, "claude-sonnet-5", SONNET),
    (
        Provider::Anthropic,
        "claude-opus-5-5",
        Price {
            input: 4.0,
            output: 20.0,
            cache_write: 5.0,
            cache_write_hour: 8.0,
            cache_read: 0.2,
        },
    ),
];

/// What a model costs, for the models this build knows ([`PRICED`]);
/// `None` for any other, which then plays only with a price given
/// ([`Price::per_million`]) or a token budget as its limit.
#[must_use]
pub fn price(model: &str) -> Option<Price> {
    PRICED
        .iter()
        .find(|(_, known, _)| *known == model)
        .map(|(_, _, price)| *price)
}

/// Tokens a provider may add to a request of its own, over the bytes sent:
/// the system prompt it writes for tools, the framing of each message.
/// Anthropic documents a few hundred for tools; this errs high.
pub const PROVIDER_ALLOWANCE: u64 = 2_000;

/// The request a first call is held to before a game: the system prompt,
/// the deck with its cards' text, and a board. A game whose budget cannot
/// pay for one such call does not sit down. The language-model seat's
/// end-to-end game holds its own first request to it.
pub const FIRST_CALL_BYTES: u64 = 64 * 1024;

/// The most tokens one call can be billed for: every byte of the request
/// as an input token (a token is at least one byte of text), the
/// provider's own [`PROVIDER_ALLOWANCE`], and a whole reply of
/// `max_tokens`. It trusts the provider to stop at `max_tokens`.
#[must_use]
pub const fn worst_tokens(request_bytes: u64, max_tokens: u32) -> u64 {
    request_bytes
        .saturating_add(PROVIDER_ALLOWANCE)
        .saturating_add(max_tokens as u64)
}

/// The most one call can cost at `price`: the input of [`worst_tokens`],
/// each token at the dearest input rate, and a whole reply at the output
/// rate.
#[must_use]
pub fn worst_usd(request_bytes: u64, max_tokens: u32, price: Price) -> f64 {
    #[allow(clippy::cast_precision_loss)] // byte counts stay far below 2^52
    let input = request_bytes.saturating_add(PROVIDER_ALLOWANCE) as f64;
    (input * price.dearest_input() + f64::from(max_tokens) * price.output) / 1_000_000.0
}

/// A price as a profile states it: input and output, in US dollars per
/// million tokens ([`Price::per_million`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GivenPrice {
    /// Input.
    pub input: f64,
    /// Output, thinking included.
    pub output: f64,
}

/// One way to play: a model and how it plays. Every field but the provider
/// and the model may be left out, and is then the build's default; a flag
/// on the bridge's command line overrides any of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// The API.
    pub provider: Provider,
    /// The model's id, as the provider names it.
    pub model: String,
    /// How hard the model thinks (`low`, `medium`, `high`, …)
    /// [default: [`Provider::default_effort`]].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// How it answers [default: [`Provider::default_answer`]]; JSON
    /// (`json`, `json_schema`) only on an OpenAI-compatible endpoint or a
    /// CLI, and a CLI only in JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<AnswerMode>,
    /// The most tokens one reply may take, thinking included [default:
    /// [`Provider::default_max_tokens`]].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// What the model costs, over this build's price for it ([`price`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<GivenPrice>,
    /// The most a game may spend, in US dollars [default:
    /// [`DEFAULT_SPEND_USD`]]; only for a model with a price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_usd: Option<f64>,
    /// The most tokens a game may spend, in and out together [default:
    /// [`DEFAULT_SPEND_TOKENS`], [`DEFAULT_CLI_SPEND_TOKENS`] for a CLI];
    /// the only limit of a model with no price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_tokens: Option<u64>,
    /// The most calls a game may make [default: [`DEFAULT_CLI_CALLS`] for a
    /// CLI, no limit for an API]; past it the house finishes the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_calls: Option<u64>,
    /// The longest one answer may take, in seconds [default:
    /// [`DEFAULT_THINK_SECS`]].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub think_secs: Option<u64>,
    /// The environment variable the key is read from [default:
    /// [`Provider::default_key_env`]]. The key itself never goes in the
    /// file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_env: Option<String>,
    /// Where the API is, over the provider's variable
    /// ([`Provider::base_env`]): the key read from [`Self::key_env`] goes
    /// to this address and no other. `https://`, or `http://` on this
    /// machine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// The program a CLI profile runs, as an absolute path [default: the
    /// tool's name found on `PATH`]. It is run directly, never through a
    /// shell, so a shell function or alias of the same name is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl Profile {
    /// A profile for `model` with every other field the build's.
    #[must_use]
    pub fn new(provider: Provider, model: &str) -> Self {
        Self {
            provider,
            model: model.to_string(),
            effort: None,
            answer: None,
            max_tokens: None,
            price: None,
            game_usd: None,
            game_tokens: None,
            game_calls: None,
            think_secs: None,
            key_env: None,
            base_url: None,
            command: None,
        }
    }

    /// What the model costs: the profile's price, else this build's; `None`
    /// when neither is known, and then no dollar can be counted for it. A
    /// CLI plays on a subscription and has none.
    #[must_use]
    pub fn price(&self) -> Option<Price> {
        if self.provider == Provider::Cli {
            return None;
        }
        self.price
            .map(|given| Price::per_million(given.input, given.output))
            .or_else(|| price(&self.model))
    }

    /// The environment variable its key is read from; `None` for a CLI,
    /// which reads no key.
    #[must_use]
    pub fn key_env(&self) -> Option<&str> {
        if self.provider == Provider::Cli {
            return None;
        }
        self.key_env
            .as_deref()
            .or_else(|| self.provider.default_key_env())
    }

    /// What is wrong with it, in one sentence, or `None`: the first of
    /// [`Self::faults`].
    #[must_use]
    pub fn fault(&self) -> Option<String> {
        self.faults().into_iter().next().map(|fault| fault.sentence)
    }

    /// Everything wrong with it, by field, each in the sentence
    /// [`Self::fault`] would say it with, in the order it looks.
    #[must_use]
    pub fn faults(&self) -> Vec<FieldFault> {
        let mut out = Vec::new();
        let mut say = |field, why, sentence: String| {
            out.push(FieldFault {
                field,
                why,
                sentence,
            });
        };
        let cli = self.provider == Provider::Cli;
        if cli {
            if let Err(sentence) = cli_model(&self.model) {
                say(Field::Model, Why::NoSuchTool, sentence);
            }
        } else if let Some(sentence) = model_fault(&self.model) {
            say(Field::Model, Why::NotAModelId, sentence);
        }
        if let Some(effort) = &self.effort
            && !effort_is_a_word(effort)
        {
            say(
                Field::Effort,
                Why::NotAWord,
                "an effort is a word such as low, medium or high".into(),
            );
        }
        if let Some((why, sentence)) = self.answer_fault() {
            say(Field::Answer, why, sentence);
        }
        if self.max_tokens == Some(0) {
            say(
                Field::MaxTokens,
                Why::Zero,
                "max_tokens is the most one reply may take: at least 1".into(),
            );
        }
        if cli {
            self.cli_faults(&mut say);
        } else {
            self.money_faults(&mut say);
        }
        if self.game_calls == Some(0) {
            say(
                Field::GameCalls,
                Why::Zero,
                "game_calls is the most calls a game may make: at least 1".into(),
            );
        }
        if self.think_secs == Some(0) {
            say(
                Field::ThinkSecs,
                Why::Zero,
                "think_secs is the longest one answer may take: at least 1".into(),
            );
        }
        if let Some(name) = self.key_env.as_ref().filter(|_| !cli)
            && !env_name_is_a_name(name)
        {
            say(
                Field::KeyEnv,
                Why::NotAVariable,
                format!(
                    "key_env names an environment variable, such as {}, and never holds the key",
                    self.provider
                        .default_key_env()
                        .unwrap_or("ANTHROPIC_API_KEY")
                ),
            );
        }
        if let Some(sentence) = self
            .base_url
            .as_deref()
            .filter(|_| !cli)
            .and_then(|base| address_fault(base, "base_url"))
        {
            say(Field::BaseUrl, Why::NotSecure, sentence);
        }
        if let Some((why, sentence)) = self.command_fault() {
            say(Field::Command, why, sentence);
        }
        out
    }
}

impl Profile {
    /// An API profile's price and dollar budget: amounts, and a dollar
    /// budget only where the model has a price.
    fn money_faults(&self, say: &mut impl FnMut(Field, Why, String)) {
        if let Some(given) = self.price {
            for (field, usd) in [
                (Field::PriceInput, given.input),
                (Field::PriceOutput, given.output),
            ] {
                if !usd_is_an_amount(usd) {
                    say(
                        field,
                        Why::NotAnAmount,
                        "a price is two amounts of US dollars per million tokens, input and \
                         output"
                            .into(),
                    );
                }
            }
        }
        if let Some(usd) = self.game_usd {
            if !usd_is_an_amount(usd) {
                say(
                    Field::GameUsd,
                    Why::NotAnAmount,
                    "game_usd is an amount of US dollars, such as 2.5".into(),
                );
            } else if self.price().is_none() {
                say(
                    Field::GameUsd,
                    Why::Unpriced,
                    format!(
                        "this build has no price for «{}», so game_usd cannot be held: give the \
                         profile a price, or make game_tokens its limit",
                        blank_key_shapes(&self.model)
                    ),
                );
            }
        }
    }

    /// Why it cannot answer the way it says, or `None`: JSON (`json`,
    /// `json_schema`) is an OpenAI-compatible endpoint's or a CLI's, and a
    /// CLI answers only in JSON.
    fn answer_fault(&self) -> Option<(Why, String)> {
        let answer = self.answer?;
        match (self.provider, answer.is_json()) {
            (Provider::Anthropic, true) => Some((
                Why::JsonNeedsOpenAi,
                format!(
                    "answer {} is for an OpenAI-compatible endpoint or a CLI; Anthropic's models \
                     answer with tools",
                    answer.name()
                ),
            )),
            (Provider::Cli, false) => Some((
                Why::NotForCli,
                "a CLI has no tools of ours and answers in JSON: answer json_schema, or json"
                    .into(),
            )),
            _ => None,
        }
    }

    /// What a CLI profile holds that a CLI does not take: a key, an
    /// address, a price in dollars. A subscription has no price, and a CLI
    /// reads no key: it plays as the player is signed in to it.
    fn cli_faults(&self, say: &mut impl FnMut(Field, Why, String)) {
        let price = "a CLI plays on a subscription, which has no price: its games are limited \
                     in game_tokens and game_calls";
        if self.price.is_some() {
            say(Field::PriceInput, Why::NotForCli, price.into());
        }
        if self.game_usd.is_some() {
            say(Field::GameUsd, Why::NotForCli, price.into());
        }
        if self.key_env.is_some() {
            say(
                Field::KeyEnv,
                Why::NotForCli,
                "a CLI reads no key: it plays as the player is signed in to it, so a cli \
                 profile has no key_env"
                    .into(),
            );
        }
        if self.base_url.is_some() {
            say(
                Field::BaseUrl,
                Why::NotForCli,
                "a CLI is a program on this machine, not an address: a cli profile has no \
                 base_url"
                    .into(),
            );
        }
    }

    /// Why its `command` cannot be run, or `None`: only a CLI profile runs
    /// a program, named by its absolute path.
    fn command_fault(&self) -> Option<(Why, String)> {
        let command = self.command.as_deref()?;
        if self.provider != Provider::Cli {
            return Some((
                Why::CliOnly,
                "command is the program a cli profile runs; an API's profile runs none".into(),
            ));
        }
        (!is_absolute_path(command)).then(|| {
            (
                Why::NotAbsolute,
                "command is the program's absolute path, such as /opt/homebrew/bin/claude".into(),
            )
        })
    }
}

/// Whether `path` is absolute on any system a bridge runs on: `/…` on unix,
/// `C:\…`, `C:/…` or `\\server\…` on Windows. Read as text, so the panel in
/// a browser says what the bridge on the player's machine would.
#[must_use]
pub fn is_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    path.starts_with('/') || path.starts_with("\\\\") || drive
}

/// A field of a profile, as the file spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    /// `provider`.
    Provider,
    /// `model`.
    Model,
    /// `effort`.
    Effort,
    /// `answer`.
    Answer,
    /// `max_tokens`.
    MaxTokens,
    /// `price.input`.
    PriceInput,
    /// `price.output`.
    PriceOutput,
    /// `game_usd`.
    GameUsd,
    /// `game_tokens`.
    GameTokens,
    /// `game_calls`.
    GameCalls,
    /// `think_secs`.
    ThinkSecs,
    /// `key_env`.
    KeyEnv,
    /// `base_url`.
    BaseUrl,
    /// `command`.
    Command,
}

impl Field {
    /// Every field, in the order a profile writes them.
    pub const ALL: [Self; 14] = [
        Self::Provider,
        Self::Model,
        Self::Effort,
        Self::Answer,
        Self::MaxTokens,
        Self::PriceInput,
        Self::PriceOutput,
        Self::GameUsd,
        Self::GameTokens,
        Self::GameCalls,
        Self::ThinkSecs,
        Self::KeyEnv,
        Self::BaseUrl,
        Self::Command,
    ];

    /// Its path in a profile, as the file spells it: a price's two amounts
    /// are a step below `price`.
    #[must_use]
    pub const fn path(self) -> &'static [&'static str] {
        match self {
            Self::Provider => &["provider"],
            Self::Model => &["model"],
            Self::Effort => &["effort"],
            Self::Answer => &["answer"],
            Self::MaxTokens => &["max_tokens"],
            Self::PriceInput => &["price", "input"],
            Self::PriceOutput => &["price", "output"],
            Self::GameUsd => &["game_usd"],
            Self::GameTokens => &["game_tokens"],
            Self::GameCalls => &["game_calls"],
            Self::ThinkSecs => &["think_secs"],
            Self::KeyEnv => &["key_env"],
            Self::BaseUrl => &["base_url"],
            Self::Command => &["command"],
        }
    }

    /// The field at `path` in a profile, if it is one.
    fn at(path: &[String]) -> Option<Self> {
        Self::ALL.into_iter().find(|field| {
            field
                .path()
                .iter()
                .copied()
                .eq(path.iter().map(String::as_str))
        })
    }
}

/// One of the [`Caps`], as the file spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CapField {
    /// `day_usd`.
    DayUsd,
    /// `month_usd`.
    MonthUsd,
    /// `day_tokens`.
    DayTokens,
    /// `month_tokens`.
    MonthTokens,
}

impl CapField {
    /// Every cap, in the order the file writes them.
    pub const ALL: [Self; 4] = [
        Self::DayUsd,
        Self::MonthUsd,
        Self::DayTokens,
        Self::MonthTokens,
    ];

    /// Its name in the file.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DayUsd => "day_usd",
            Self::MonthUsd => "month_usd",
            Self::DayTokens => "day_tokens",
            Self::MonthTokens => "month_tokens",
        }
    }

    /// The cap of that name.
    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|cap| cap.name() == name)
    }
}

/// A calendar period the [`Caps`] count over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Period {
    /// A calendar day.
    Day,
    /// A calendar month.
    Month,
}

/// Where in the settings a [`Fault`] is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// The name of the default profile.
    Default,
    /// The name of a profile.
    Name(String),
    /// A field of the profile of that name.
    Profile(String, Field),
    /// A cap.
    Cap(CapField),
    /// Anywhere else, by its path in the file: a field the file does not
    /// have, which only a text can hold and a [`SeatSettings`] never does.
    Elsewhere(String),
}

impl Place {
    /// The place at `steps` in the file, which the file spells `path`.
    fn at(steps: &[String], path: &str) -> Self {
        let elsewhere = || Self::Elsewhere(clip(path));
        match steps {
            [top] if top == "default" => Self::Default,
            [top, name] if top == "profiles" => Self::Name(name.clone()),
            [top, name, rest @ ..] if top == "profiles" => {
                Field::at(rest).map_or_else(elsewhere, |field| Self::Profile(name.clone(), field))
            }
            [top, cap] if top == "caps" => CapField::named(cap).map_or_else(elsewhere, Self::Cap),
            _ => elsewhere(),
        }
    }
}

/// What is wrong, as a [`Fault`] says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Why {
    /// A field named like a key.
    KeyNamed,
    /// A value, or a name, shaped like a key.
    KeyShaped,
    /// The default names no profile.
    NoSuchProfile,
    /// A profile's name that a command line cannot carry.
    NotAName,
    /// A model id no provider writes.
    NotAModelId,
    /// A CLI's model that names no tool this build plays.
    NoSuchTool,
    /// An effort that is not one word.
    NotAWord,
    /// JSON answers (`json`, `json_schema`) asked of Anthropic's models.
    JsonNeedsOpenAi,
    /// What a CLI does not take: a key, an address, a price, answers by
    /// tool.
    NotForCli,
    /// A program to run named on a profile that is not a CLI's.
    CliOnly,
    /// A program named by a path that is not absolute.
    NotAbsolute,
    /// A most of zero: `max_tokens`, `game_calls` or `think_secs`.
    Zero,
    /// Dollars that are not an amount: below zero, or not finite.
    NotAnAmount,
    /// A game's dollar budget for a model with no price.
    Unpriced,
    /// A key variable that is not a variable's name.
    NotAVariable,
    /// An address a key may not be sent to.
    NotSecure,
    /// Settings that cannot be written as JSON at all.
    Unwritable,
}

/// One thing the settings may not hold: where it is, what it is, and the
/// sentence [`SeatSettings::check`] refuses it with. A panel puts each
/// beside its field in the player's language; the bridge says the sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    /// Where.
    pub place: Place,
    /// What.
    pub why: Why,
    /// The refusal, in one sentence.
    pub sentence: String,
}

/// One thing a [`Profile`] may not hold, by field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldFault {
    /// The field.
    pub field: Field,
    /// What.
    pub why: Why,
    /// The refusal, in one sentence, without the profile's name.
    pub sentence: String,
}

/// The most a player's games may spend together, per calendar day and per
/// month, over every profile (`docs/llm-seat.md` §"The spend book"). Dollars
/// count the games of models with a price; tokens count the games of models
/// without one, whose dollars nobody can count. None set, no cap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caps {
    /// US dollars a day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_usd: Option<f64>,
    /// US dollars a month.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month_usd: Option<f64>,
    /// Tokens a day, for models with no price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_tokens: Option<u64>,
    /// Tokens a month, for models with no price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month_tokens: Option<u64>,
}

impl Caps {
    /// Whether no cap is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Why a model with no price cannot play under these caps, or `None`:
    /// a period capped in dollars needs a token cap too, or that model's
    /// spend would go uncounted in it.
    #[must_use]
    pub fn unpriced_fault(&self, model: &str) -> Option<String> {
        self.unpriced_period().map(|period| {
            let (period, field) = match period {
                Period::Day => ("day", "day_tokens"),
                Period::Month => ("month", "month_tokens"),
            };
            format!(
                "the settings file caps dollars a {period}, and this build has no price for \
                 «{model}», so its spend cannot be counted in dollars: give its profile a price, \
                 or the caps a {field}"
            )
        })
    }

    /// The first period capped in dollars and not in tokens, in which a
    /// model with no price could not be counted ([`Self::unpriced_fault`]).
    #[must_use]
    pub fn unpriced_period(&self) -> Option<Period> {
        [
            (self.day_usd, self.day_tokens, Period::Day),
            (self.month_usd, self.month_tokens, Period::Month),
        ]
        .into_iter()
        .find(|(usd, tokens, _)| usd.is_some() && tokens.is_none())
        .map(|(_, _, period)| period)
    }

    /// The caps in dollars that are not an amount.
    fn faults(&self) -> impl Iterator<Item = Fault> {
        [
            (CapField::DayUsd, self.day_usd),
            (CapField::MonthUsd, self.month_usd),
        ]
        .into_iter()
        .filter(|(_, usd)| usd.is_some_and(|usd| !usd_is_an_amount(usd)))
        .map(|(cap, _)| Fault {
            place: Place::Cap(cap),
            why: Why::NotAnAmount,
            sentence: "a cap in dollars is an amount of US dollars, such as 20".into(),
        })
    }
}

/// The settings file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatSettings {
    /// The profile the bridge plays when told neither a mind nor a profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// What every game counts against, across games.
    #[serde(default, skip_serializing_if = "Caps::is_empty")]
    pub caps: Caps,
    /// The ways to play, by name.
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

impl SeatSettings {
    /// Reads the file's text.
    ///
    /// # Errors
    /// In one sentence: for text that is not the file's JSON, a field it
    /// does not have (every unknown field is refused), a field named like
    /// a key or a name or value shaped like one (said without the value),
    /// and for what [`Self::check`] refuses.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
        if let Some(fault) = keys_in(&value).into_iter().next() {
            return Err(fault.sentence);
        }
        let settings: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        settings.check()?;
        Ok(settings)
    }

    /// The file's text: JSON, two-space indented, fields left out where
    /// they are the build's.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        text.push('\n');
        text
    }

    /// Whether the settings hold together, and hold no key: everything
    /// [`SeatSettings::parse`] refuses of a file, so that nothing it would
    /// refuse is ever written (a key pasted into a model's name, a profile
    /// called `api_key`).
    ///
    /// # Errors
    /// A sentence naming the profile or the field, the first of
    /// [`Self::faults`]: a field named like a key or a name or value shaped
    /// like one, a default that names no profile, a name a command line
    /// cannot carry, a profile's [`Profile::fault`], a cap that is not an
    /// amount.
    pub fn check(&self) -> Result<(), String> {
        match self.faults().into_iter().next() {
            Some(fault) => Err(fault.sentence),
            None => Ok(()),
        }
    }

    /// Everything [`Self::check`] refuses, each where it is and in the
    /// sentence `check` says it with, in the order it looks: keys first,
    /// then the default, each profile's name and fields, and the caps.
    #[must_use]
    pub fn faults(&self) -> Vec<Fault> {
        let mut out = match serde_json::to_value(self) {
            Ok(value) => keys_in(&value),
            Err(e) => vec![Fault {
                place: Place::Elsewhere(String::new()),
                why: Why::Unwritable,
                sentence: e.to_string(),
            }],
        };
        if let Some(default) = &self.default
            && !self.profiles.contains_key(default)
        {
            out.push(Fault {
                place: Place::Default,
                why: Why::NoSuchProfile,
                sentence: format!(
                    "the default profile «{}» is not among the profiles{}",
                    clip(default),
                    self.known()
                ),
            });
        }
        for (name, profile) in &self.profiles {
            if !profile_name_is_a_name(name) {
                out.push(Fault {
                    place: Place::Name(name.clone()),
                    why: Why::NotAName,
                    sentence: format!(
                        "«{}» is not a profile name: up to 32 letters, digits, - and _, starting \
                         with a letter or digit",
                        clip(name)
                    ),
                });
            }
            out.extend(profile.faults().into_iter().map(|fault| Fault {
                place: Place::Profile(name.clone(), fault.field),
                why: fault.why,
                sentence: format!("profile «{}»: {}", clip(name), fault.sentence),
            }));
        }
        out.extend(self.caps.faults());
        out
    }

    /// The profile called `name`.
    #[must_use]
    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// ": it has a, b" for a sentence about a name it lacks, or "; it has
    /// none".
    #[must_use]
    pub fn known(&self) -> String {
        if self.profiles.is_empty() {
            "; it has none".into()
        } else {
            let names: Vec<String> = self.profiles.keys().map(|name| clip(name)).collect();
            format!(": it has {}", names.join(", "))
        }
    }
}

/// What a sentence about keys in the file ends with.
const KEYS_GO: &str = "a key never goes in the settings file: set it in the environment and name \
                       the variable with key_env (ANTHROPIC_API_KEY by default, \
                       BAYLEE_LLM_API_KEY for openai; a cli profile reads none)";

/// Field names someone writes a key under. Exact names only, and names
/// ending in `api_key`: the file's own fields (`game_tokens`, `max_tokens`,
/// `key_env`) are none of them.
fn key_like(name: &str) -> bool {
    let name = name.to_ascii_lowercase().replace('-', "_");
    matches!(
        name.as_str(),
        "key"
            | "apikey"
            | "secret"
            | "token"
            | "bearer"
            | "password"
            | "authorization"
            | "access_token"
            | "auth_token"
            | "x_api_key"
    ) || name.ends_with("api_key")
}

/// Every field named like a key, and every name or value shaped like one,
/// in `value`, in the order they stand, as the faults that refuse them.
fn keys_in(value: &Value) -> Vec<Fault> {
    let mut out = Vec::new();
    keys_below(value, &mut Vec::new(), "", &mut out);
    out
}

/// [`keys_in`] below `steps`, which the file spells `path`. A field named
/// like a key is refused whole: what it holds is not looked into.
fn keys_below(value: &Value, steps: &mut Vec<String>, path: &str, out: &mut Vec<Fault>) {
    let refuse = |steps: &[String], path: &str, why| {
        let sentence = match why {
            Why::KeyNamed => format!("«{}» in the settings file: {KEYS_GO}", clip(path)),
            _ => format!(
                "{} in the settings file looks like an API key: {KEYS_GO}",
                clip(path)
            ),
        };
        Fault {
            place: Place::at(steps, path),
            why,
            sentence,
        }
    };
    match value {
        Value::Object(fields) => {
            for (name, inner) in fields {
                let here = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                steps.push(name.clone());
                if key_like(name) {
                    out.push(refuse(steps, &here, Why::KeyNamed));
                } else if shaped_like_a_key(name) {
                    // A name is written to the file as surely as a value:
                    // a profile called by a key would carry it there.
                    out.push(refuse(steps, &here, Why::KeyShaped));
                } else {
                    keys_below(inner, steps, &here, out);
                }
                steps.pop();
            }
        }
        Value::Array(items) => {
            for (at, inner) in items.iter().enumerate() {
                steps.push(format!("[{at}]"));
                keys_below(inner, steps, &format!("{path}[{at}]"), out);
                steps.pop();
            }
        }
        Value::String(text) if shaped_like_a_key(text) => {
            out.push(refuse(steps, path, Why::KeyShaped));
        }
        _ => {}
    }
}

/// `text` with each run shaped like an API key blanked: `sk-` followed by
/// sixteen or more key characters (Anthropic's `sk-ant-…`, `OpenAI`'s and
/// `DeepSeek`'s `sk-…`), a GitHub token's prefix followed by twenty or
/// more, and whatever follows `Bearer ` or `x-api-key` up to the next space
/// or quote. Unlike [`shaped_like_a_key`] it blanks inside a word too: a
/// redaction errs towards blanking. The markers stay, so a reader still
/// sees what was there. The seat bridge's scrubber blanks these and its own
/// key.
#[must_use]
pub fn blank_key_shapes(text: &str) -> String {
    let mut out = blank_after(text, "sk-", 16);
    for marker in ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_"] {
        out = blank_after(&out, marker, 20);
    }
    for marker in ["Bearer ", "bearer ", "x-api-key: ", "x-api-key\":\""] {
        out = blank_after(&out, marker, 1);
    }
    out
}

/// Whether `text` holds anything shaped like a key: a marker with as many
/// key characters after it as such a key has, or an authorization header's
/// words.
///
/// Two kinds of marker. The **generic** ones (`sk-`, `Bearer `, `x-api-key`)
/// are short and common inside words, so they count only where a word
/// starts: glued to a lowercase word before it (`desk-tools-collection`,
/// `risk-free`: a lowercase letter, `_`, `-` or `.` before it) they are part
/// of that word and no key. Anything else before one starts a word: the
/// text's start, a space, `/`, `=`, an uppercase letter, a non-ASCII
/// character, and a digit, since a model id ends in one and a key pasted
/// after it (`claude-sonnet-5-5sk-…`) starts where its marker does.
///
/// The **provider-specific** ones (`sk-ant-`, `sk-proj-`, GitHub's `ghp_`,
/// `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`) are long and followed by at
/// least twenty key characters, which no word is, so they count wherever
/// they stand: a key pasted onto a model id (`sonnetsk-ant-…`) is still one,
/// while `task-ant-hill` is too short after its marker to be.
///
/// The one definition: the settings file and panel refuse by it, and the
/// seat bridge refuses to start a CLI whose environment holds such a value.
#[must_use]
pub fn shaped_like_a_key(text: &str) -> bool {
    /// Marker, key characters it needs after it, and whether it must start
    /// a word.
    const MARKERS: [(&str, usize, bool); 12] = [
        ("sk-", 16, true),
        ("sk-ant-", 20, false),
        ("sk-proj-", 20, false),
        ("ghp_", 20, false),
        ("gho_", 20, false),
        ("ghu_", 20, false),
        ("ghs_", 20, false),
        ("ghr_", 20, false),
        ("github_pat_", 20, false),
        ("Bearer ", 1, true),
        ("bearer ", 1, true),
        ("x-api-key", 0, true),
    ];
    MARKERS.iter().any(|(marker, least, word)| {
        text.match_indices(marker).any(|(at, _)| {
            let starts_a_word = text[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_ascii_lowercase() || matches!(c, '_' | '-' | '.')));
            let run = text[at + marker.len()..]
                .chars()
                .take_while(|c| key_char(*c))
                .count();
            (starts_a_word || !word) && run >= *least
        })
    })
}

/// A character a key is written in.
fn key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

/// Blanks each run of key characters that follows `marker`, when the run is
/// at least `least` long.
fn blank_after(text: &str, marker: &str, least: usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(marker) {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        out.push_str(marker);
        let tail = &after[marker.len()..];
        let run = tail
            .char_indices()
            .find(|(_, c)| !key_char(*c))
            .map_or(tail.len(), |(i, _)| i);
        if run >= least {
            out.push_str("[redacted]");
        } else {
            out.push_str(&tail[..run]);
        }
        rest = &tail[run..];
    }
    out.push_str(rest);
    out
}

/// Why `model` is not a model id, or `None`: an id is up to 100 of the
/// characters providers use (letters, digits, `-_.:/@`).
#[must_use]
pub fn model_fault(model: &str) -> Option<String> {
    let valid = !model.is_empty()
        && model.len() <= 100
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.:/@".contains(c));
    (!valid).then(|| {
        format!(
            "«{}» is not a model id",
            blank_key_shapes(model).chars().take(40).collect::<String>()
        )
    })
}

/// Whether `effort` is a word such as `low` or `high`.
#[must_use]
pub fn effort_is_a_word(effort: &str) -> bool {
    !effort.is_empty() && effort.len() <= 10 && effort.chars().all(|c| c.is_ascii_lowercase())
}

/// Whether `usd` is an amount of US dollars: finite and not below zero.
#[must_use]
pub fn usd_is_an_amount(usd: f64) -> bool {
    usd.is_finite() && usd >= 0.0
}

/// Whether `name` is an environment variable's name: capitals, digits and
/// `_`, not starting with a digit, at most 64.
fn env_name_is_a_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Whether `name` is a profile's name: a word a command line carries as
/// `--profile <name>` and `--bridge profile:<name>`.
#[must_use]
pub fn profile_name_is_a_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Why `base` is not an address a key may be sent to, naming it as
/// `what`, or `None`: `https://`, or `http://` only on this machine.
#[must_use]
pub fn address_fault(base: &str, what: &str) -> Option<String> {
    let secure = base.starts_with("https://");
    let local = is_loopback(base) && base.starts_with("http://");
    (!(secure || local))
        .then(|| format!("{what} must be an https:// address (http:// only on this machine)"))
}

/// Whether `base` is this machine.
#[must_use]
pub fn is_loopback(base: &str) -> bool {
    let host = base
        .split_once("://")
        .map_or(base, |(_, rest)| rest)
        .split(['/', '?'])
        .next()
        .unwrap_or_default();
    let host = host
        .rsplit_once(':')
        .filter(|(h, port)| !h.is_empty() && port.chars().all(|c| c.is_ascii_digit()))
        .map_or(host, |(h, _)| h);
    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

/// `name` cut short for a sentence, and with any key shape blanked.
fn clip(name: &str) -> String {
    blank_key_shapes(name).chars().take(40).collect()
}
