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
//! where keys go ([`SeatSettings::parse`]).
//!
//! Pure data and arithmetic, for every target. Reading and writing the
//! files is [`store`] and [`ledger::Book`], on native targets only, the one
//! place in this crate that touches a file. Nothing here reads a clock: the
//! ledger is handed the moment ([`ledger::Moment`]).

pub mod ledger;
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
}

impl Provider {
    /// The name `--mind` and the file spell it with.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }

    /// The environment variable its key is read from unless a profile names
    /// another.
    #[must_use]
    pub const fn default_key_env(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::OpenAi => "BAYLEE_LLM_API_KEY",
        }
    }

    /// The environment variable its address is read from unless a profile
    /// names one.
    #[must_use]
    pub const fn base_env(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_BASE_URL",
            Self::OpenAi => "BAYLEE_LLM_BASE_URL",
        }
    }

    /// Its address when neither a profile nor the environment names one.
    #[must_use]
    pub const fn default_base(self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::OpenAi => "https://api.openai.com/v1",
        }
    }

    /// The most tokens one reply may take, thinking included, when nothing
    /// says otherwise.
    #[must_use]
    pub const fn default_max_tokens(self) -> u32 {
        match self {
            Self::Anthropic => 16_000,
            Self::OpenAi => 8_000,
        }
    }

    /// How hard its models think when nothing says otherwise: medium on
    /// Anthropic, the endpoint's own default elsewhere.
    #[must_use]
    pub const fn default_effort(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("medium"),
            Self::OpenAi => None,
        }
    }
}

/// How the model answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerMode {
    /// By calling the `decide` or `concede` tool.
    Tools,
    /// With one JSON object, for an endpoint without tools.
    Json,
}

/// A model's price per million tokens, in US dollars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Price {
    /// Input.
    pub input: f64,
    /// Output, thinking included.
    pub output: f64,
    /// Input written to the cache.
    pub cache_write: f64,
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
            cache_read: input,
        }
    }

    /// The dearest way an input token can be billed: plain, written to the
    /// cache, or read from it.
    #[must_use]
    pub fn dearest_input(&self) -> f64 {
        self.input.max(self.cache_write).max(self.cache_read)
    }
}

/// What a model costs, for the models this build knows; `None` for any
/// other, which then plays only with a price given ([`Price::per_million`])
/// or a token budget as its limit. A snapshot: the provider's price list is
/// the authority.
#[must_use]
pub fn price(model: &str) -> Option<Price> {
    match model {
        "claude-sonnet-5-5" | "claude-sonnet-5" => Some(Price {
            input: 2.0,
            output: 10.0,
            cache_write: 2.5,
            cache_read: 0.2,
        }),
        "claude-opus-5-5" => Some(Price {
            input: 4.0,
            output: 20.0,
            cache_write: 5.0,
            cache_read: 0.2,
        }),
        _ => None,
    }
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
    /// How it answers [default: tools]; JSON only on an OpenAI-compatible
    /// endpoint.
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
    /// [`DEFAULT_SPEND_TOKENS`]]; the only limit of a model with no price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_tokens: Option<u64>,
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
            think_secs: None,
            key_env: None,
            base_url: None,
        }
    }

    /// What the model costs: the profile's price, else this build's; `None`
    /// when neither is known, and then no dollar can be counted for it.
    #[must_use]
    pub fn price(&self) -> Option<Price> {
        self.price
            .map(|given| Price::per_million(given.input, given.output))
            .or_else(|| price(&self.model))
    }

    /// The environment variable its key is read from.
    #[must_use]
    pub fn key_env(&self) -> &str {
        self.key_env
            .as_deref()
            .unwrap_or_else(|| self.provider.default_key_env())
    }

    /// What is wrong with it, in one sentence, or `None`.
    #[must_use]
    pub fn fault(&self) -> Option<String> {
        if let Some(why) = model_fault(&self.model) {
            return Some(why);
        }
        if let Some(effort) = &self.effort
            && !effort_is_a_word(effort)
        {
            return Some("an effort is a word such as low, medium or high".into());
        }
        if self.answer == Some(AnswerMode::Json) && self.provider != Provider::OpenAi {
            return Some(
                "answer json is for an OpenAI-compatible endpoint; Anthropic's models answer \
                 with tools"
                    .into(),
            );
        }
        if self.max_tokens == Some(0) {
            return Some("max_tokens is the most one reply may take: at least 1".into());
        }
        if let Some(given) = self.price
            && !(usd_is_an_amount(given.input) && usd_is_an_amount(given.output))
        {
            return Some(
                "a price is two amounts of US dollars per million tokens, input and output".into(),
            );
        }
        if let Some(usd) = self.game_usd {
            if !usd_is_an_amount(usd) {
                return Some("game_usd is an amount of US dollars, such as 2.5".into());
            }
            if self.price().is_none() {
                return Some(format!(
                    "this build has no price for «{}», so game_usd cannot be held: give the \
                     profile a price, or make game_tokens its limit",
                    self.model
                ));
            }
        }
        if self.think_secs == Some(0) {
            return Some("think_secs is the longest one answer may take: at least 1".into());
        }
        if let Some(name) = &self.key_env
            && !env_name_is_a_name(name)
        {
            return Some(format!(
                "key_env names an environment variable, such as {}, and never holds the key",
                self.provider.default_key_env()
            ));
        }
        if let Some(base) = &self.base_url {
            return address_fault(base, "base_url");
        }
        None
    }
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
        let missing = [
            (self.day_usd, self.day_tokens, "day", "day_tokens"),
            (self.month_usd, self.month_tokens, "month", "month_tokens"),
        ]
        .into_iter()
        .find(|(usd, tokens, _, _)| usd.is_some() && tokens.is_none());
        missing.map(|(_, _, period, field)| {
            format!(
                "the settings file caps dollars a {period}, and this build has no price for \
                 «{model}», so its spend cannot be counted in dollars: give its profile a price, \
                 or the caps a {field}"
            )
        })
    }

    fn fault(&self) -> Option<String> {
        [self.day_usd, self.month_usd]
            .into_iter()
            .flatten()
            .any(|usd| !usd_is_an_amount(usd))
            .then(|| "a cap in dollars is an amount of US dollars, such as 20".to_string())
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
    /// a key or a value shaped like one (said without the value), and for
    /// what [`Self::check`] refuses.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
        if let Some(why) = key_in(&value, "") {
            return Err(why);
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
    /// A sentence naming the profile or the field: a field named like a key
    /// or a value shaped like one, a default that names no profile, a name
    /// a command line cannot carry, a profile's [`Profile::fault`], a cap
    /// that is not an amount.
    pub fn check(&self) -> Result<(), String> {
        let value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        if let Some(why) = key_in(&value, "") {
            return Err(why);
        }
        if let Some(default) = &self.default
            && !self.profiles.contains_key(default)
        {
            return Err(format!(
                "the default profile «{}» is not among the profiles{}",
                clip(default),
                self.known()
            ));
        }
        for (name, profile) in &self.profiles {
            if !profile_name_is_a_name(name) {
                return Err(format!(
                    "«{}» is not a profile name: up to 32 letters, digits, - and _, starting \
                     with a letter or digit",
                    clip(name)
                ));
            }
            if let Some(why) = profile.fault() {
                return Err(format!("profile «{name}»: {why}"));
            }
        }
        if let Some(why) = self.caps.fault() {
            return Err(why);
        }
        Ok(())
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
            let names: Vec<&str> = self.profiles.keys().map(String::as_str).collect();
            format!(": it has {}", names.join(", "))
        }
    }
}

/// What a sentence about keys in the file ends with.
const KEYS_GO: &str = "a key never goes in the settings file: set it in the environment and name \
                       the variable with key_env (ANTHROPIC_API_KEY by default, \
                       BAYLEE_LLM_API_KEY for openai)";

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

/// The first field named like a key, or value shaped like one, in `value`
/// at `path`, as the sentence that refuses it.
fn key_in(value: &Value, path: &str) -> Option<String> {
    match value {
        Value::Object(fields) => fields.iter().find_map(|(name, inner)| {
            let here = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if key_like(name) {
                return Some(format!("«{}» in the settings file: {KEYS_GO}", clip(&here)));
            }
            key_in(inner, &here)
        }),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .find_map(|(at, inner)| key_in(inner, &format!("{path}[{at}]"))),
        Value::String(text) if shaped_like_a_key(text) => Some(format!(
            "{} in the settings file looks like an API key: {KEYS_GO}",
            clip(path)
        )),
        _ => None,
    }
}

/// `text` with each run shaped like an API key blanked: `sk-` followed by
/// sixteen or more key characters (Anthropic's `sk-ant-…`, `OpenAI`'s and
/// `DeepSeek`'s `sk-…`), and whatever follows `Bearer ` or `x-api-key` up
/// to the next space or quote. The markers stay, so a reader still sees
/// what was there. The seat bridge's scrubber blanks these and its own key.
#[must_use]
pub fn blank_key_shapes(text: &str) -> String {
    let mut out = blank_after(text, "sk-", 16);
    for marker in ["Bearer ", "bearer ", "x-api-key: ", "x-api-key\":\""] {
        out = blank_after(&out, marker, 1);
    }
    out
}

/// Whether `text` holds anything shaped like an API key.
#[must_use]
pub fn shaped_like_a_key(text: &str) -> bool {
    blank_key_shapes(text) != text
}

/// Blanks each run of key characters that follows `marker`, when the run is
/// at least `least` long.
fn blank_after(text: &str, marker: &str, least: usize) -> String {
    let key_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.';
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
