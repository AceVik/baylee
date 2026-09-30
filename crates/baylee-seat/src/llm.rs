//! A language model behind an API, as a mind (`llm-seat.md` §4.4–§4.6, §8).
//!
//! [`ApiMind`] tells each real decision as one message ([`crate::narrator`]),
//! sends it to the model with two tools, `decide` and `concede`, and reads
//! the call back against the question's own options ([`Menu::resolve`]).
//! Two providers: the Anthropic Messages API and any OpenAI-compatible chat
//! endpoint (`DeepSeek`, `OpenAI`, a local server), the latter also without
//! tools, answering one JSON object ([`AnswerMode::Json`]).
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
mod secret;
#[cfg(test)]
mod tests;

pub use secret::{Secret, scrub};

use crate::mind::{Answer, Disclosure, Mind, MindError, Readiness, Request, Thinking};
use crate::narrator::{self, Act, Decision, Hint, Menu, Narrator};
use crate::transcript::Transcript;
use baylee_client_core::llmseat::{
    address_fault, is_loopback, model_fault, worst_tokens, worst_usd,
};
use baylee_client_core::manaplan;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction, TargetPrompt};
use baylee_view::{LogEvent, LogObject};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// The model's price and the build's defaults live beside the settings
/// file's types, so the client's panel shows what the bridge plays with.
pub use baylee_client_core::llmseat::{
    AnswerMode, DEFAULT_ANTHROPIC_MODEL, DEFAULT_SPEND_TOKENS, DEFAULT_SPEND_USD, Price, Provider,
    price,
};

/// The least time worth a second call after an unreadable answer.
const RETRY_FLOOR: Duration = Duration::from_secs(5);

/// How much sooner than the bridge's deadline a call gives up, so the
/// answer, if any, still reaches the seat.
const MARGIN: Duration = Duration::from_secs(1);

/// The most earlier notes a new turn's conversation carries.
const SAYS: usize = 8;

/// A model, as `--mind` names it: `anthropic`, `anthropic:<model>` or
/// `openai:<model>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The API.
    pub provider: Provider,
    /// The model's id.
    pub model: String,
}

impl Spec {
    /// Reads `anthropic[:<model>]` or `openai:<model>`; `None` for anything
    /// else (the other minds).
    ///
    /// # Errors
    /// For `openai` without a model, or a model id with characters no
    /// provider uses.
    pub fn parse(text: &str) -> Option<Result<Self, String>> {
        let (provider, model) = match text.split_once(':') {
            Some((provider, model)) => (provider, Some(model)),
            None => (text, None),
        };
        let provider = match provider {
            "anthropic" => Provider::Anthropic,
            "openai" => Provider::OpenAi,
            _ => return None,
        };
        let model = match (provider, model) {
            (Provider::Anthropic, None) => DEFAULT_ANTHROPIC_MODEL.to_string(),
            (Provider::OpenAi, None) => {
                return Some(Err("name the model: openai:<model>".into()));
            }
            (_, Some(model)) => model.trim().to_string(),
        };
        if let Some(why) = model_fault(&model) {
            return Some(Err(why));
        }
        Some(Ok(Self { provider, model }))
    }

    /// The name a chair it plays sits under, after `LLM-`: the model's id
    /// without `claude-`, in the characters a seat name allows, at most
    /// twelve.
    #[must_use]
    pub fn tag(&self) -> String {
        let bare = self.model.strip_prefix("claude-").unwrap_or(&self.model);
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
}

/// How an [`ApiMind`] plays.
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
    /// How long a turn's conversation may grow, in estimated tokens, before
    /// the next question starts a new one with the notes carried over: a
    /// long turn neither outgrows the model's context nor has every call
    /// send all of it again.
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
    /// dollars where this build knows the model's price.
    #[must_use]
    pub fn new(spec: &Spec) -> Self {
        let price = price(&spec.model);
        Self {
            provider: spec.provider,
            model: spec.model.clone(),
            effort: spec.provider.default_effort().map(str::to_string),
            answer: AnswerMode::Tools,
            max_tokens: spec.provider.default_max_tokens(),
            price,
            spend_usd: price.map(|_| DEFAULT_SPEND_USD),
            spend_tokens: DEFAULT_SPEND_TOKENS,
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
    /// never with a dollar budget.
    pub fn budget(
        &mut self,
        price: Option<Price>,
        spend_usd: Option<f64>,
        spend_tokens: Option<u64>,
    ) -> Result<(), String> {
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

/// Where the API is and the key it takes.
#[derive(Clone, Debug)]
pub struct Credentials {
    key: Option<Secret>,
    base: String,
}

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
    credentials_at(provider, provider.default_key_env(), None, env)
}

/// The key in the environment variable `key_env`, for the address `base`
/// where a profile names one, else the provider's variable
/// ([`Provider::base_env`]), else the provider's own address. A key named
/// beside an address goes to that address and no other, so a profile's
/// `base_url` wins over the variable.
///
/// # Errors
/// As [`credentials`].
pub fn credentials_at(
    provider: Provider,
    key_env: &str,
    base: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Credentials, String> {
    let key = env(key_env).as_deref().and_then(Secret::new);
    let tidy = |b: &str| b.trim().trim_end_matches('/').to_string();
    let (base, named) = match base {
        Some(base) => (tidy(base), "base_url"),
        None => (
            env(provider.base_env())
                .map(|b| tidy(&b))
                .filter(|b| !b.is_empty())
                .unwrap_or_else(|| provider.default_base().to_string()),
            provider.base_env(),
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
    Ok(Credentials { key, base })
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

    /// Holds `worst` for a call about to be sent, unless it could pass the
    /// game's hard limit ([`Settings::hard_limit`]); then the sentence why.
    fn hold(&mut self, worst: Worst, settings: &Settings) -> Result<(), String> {
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
        self.held.add(worst);
        Ok(())
    }

    /// Takes back what [`Self::hold`] held for a call that came back, and
    /// counts how it came back: its tokens as the provider counted them, or
    /// its worst when the bill is unknown.
    fn back(&mut self, worst: Worst, billed: Result<Usage, bool>, price: Option<Price>) {
        self.held.sub(worst);
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

/// Taps still to send, and what they pay for.
#[derive(Clone, Debug)]
struct Plan {
    steps: VecDeque<manaplan::Step>,
    then: PlayerAction,
    /// The option, in words.
    label: String,
    /// The step it was chosen in: a plan never runs into another.
    at: (u32, baylee_view::Phase, baylee_view::Step),
    /// The colour the last tap makes, for the question it may ask.
    color: Option<ManaColor>,
}

/// What one seat's conversation holds between decisions.
struct SeatState {
    narrator: Narrator,
    /// The conversation, as the provider takes it.
    messages: Vec<Value>,
    /// The turn the conversation belongs to; 0 before the first.
    turn: u32,
    /// What the seat owes the model's last tool calls.
    results: Vec<ToolResult>,
    /// Whether the last answer sent was the model's own (not a plan's).
    last_by_model: bool,
    plan: Option<Plan>,
    hint: Option<Hint>,
    /// Lines for the next message, under its header.
    notes: Vec<String>,
    /// The model's own earlier `say` lines, newest last.
    says: VecDeque<String>,
    /// A question whose answer came after its time.
    late: Option<u64>,
    /// How many calls this seat has started, to tell a stale one.
    asked: u64,
    /// The card this seat last answered a cast of, until the log or the
    /// next priority shows whether the cast happened
    /// ([`SeatState::undone`]).
    casting: Option<ObjectId>,
    transcript: Transcript,
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
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
        Self {
            settings,
            credentials,
            agent,
            seats: Mutex::new(BTreeMap::new()),
            tally: Arc::new(Mutex::new(Tally::default())),
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
            let transcript =
                self.settings
                    .transcripts
                    .as_ref()
                    .map_or_else(Transcript::none, |dir| {
                        let path = dir.join(format!(
                            "{}-seat{}-mind.jsonl",
                            context.game_id,
                            context.seat.get()
                        ));
                        Transcript::file(&path).unwrap_or_else(|_| Transcript::none())
                    });
            Arc::new(Mutex::new(SeatState {
                narrator: Narrator::new(context),
                messages: Vec::new(),
                turn: 0,
                results: Vec::new(),
                last_by_model: false,
                plan: None,
                hint: None,
                notes: Vec::new(),
                says: VecDeque::new(),
                late: None,
                asked: 0,
                casting: None,
                transcript,
            }))
        }))
    }

    /// Why the game's budget is spent, if it is.
    fn spent(&self) -> Option<String> {
        let mut tally = lock(&self.tally);
        // Under a hard limit, the calls still out and those whose bill is
        // unknown count at their worst.
        let (tokens, usd) = if self.settings.hard_limit {
            (tally.spend_tokens(), tally.spend_usd().unwrap_or(0.0))
        } else {
            (tally.usage.total(), tally.usd.unwrap_or(0.0))
        };
        let why = if tokens >= self.settings.spend_tokens {
            format!(
                "the game's budget of {} tokens is spent",
                self.settings.spend_tokens
            )
        } else if let Some(budget) = self.settings.spend_usd
            && usd >= budget
        {
            format!("the game's budget of ${budget:.2} is spent (${usd:.2})")
        } else {
            return None;
        };
        tally.spent = true;
        Some(why)
    }

    /// The request's headers, the key among them.
    fn headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![("content-type", "application/json".to_string())];
        match self.settings.provider {
            Provider::Anthropic => {
                headers.push(("anthropic-version", anthropic::VERSION.to_string()));
                if let Some(key) = &self.credentials.key {
                    headers.push(("x-api-key", key.expose().to_string()));
                }
            }
            Provider::OpenAi => {
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
        let url = match self.settings.provider {
            Provider::Anthropic => anthropic::url(&self.credentials.base),
            Provider::OpenAi => openai::url(&self.credentials.base),
        };
        let headers = self.headers();
        let agent = self.agent.clone();
        let tally = Arc::clone(&self.tally);
        let price = self.settings.price;
        let provider = self.settings.provider;
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
            let result = call(&agent, &url, &headers, &bytes, timeout, provider, mode);
            let billed = match &result {
                Ok(reply) => Ok(reply.usage),
                Err(failed) => Err(failed.unknown_bill),
            };
            lock(&tally).back(worst, billed, price);
            if !alive.load(Ordering::SeqCst) && result.is_ok() {
                lock(&seat).late = Some(question);
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
            let body = match self.settings.provider {
                Provider::Anthropic => anthropic::body(&self.settings, &messages),
                Provider::OpenAi => openai::body(&self.settings, &messages),
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
                    if state.asked != prepared.asked {
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
                        self.settings.provider,
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
    fn start(
        &self,
        seat: &Mutex<SeatState>,
        request: &Request,
    ) -> Result<Prepared, Result<Answer, MindError>> {
        let mut state = lock(seat);
        state.narrator.hear(&request.log);
        if let Some(undone) = state.undone(request) {
            state.notes.push(undone);
        }
        if let Some(question) = state.late.take() {
            state.notes.push(format!(
                "Your answer to q{question} came after its time ran out; the house answered it."
            ));
        }
        if let Some(refusal) = &request.retry {
            state.refused(&refusal.reason, &refusal.answer, self.settings.answer);
        }
        if let Some((action, label)) = state.follow(request) {
            state.last_by_model = false;
            let note = json!({"plan": label}).to_string();
            return Err(Ok(Answer {
                action,
                model_time: Duration::ZERO,
                note: Some(note),
            }));
        }
        if let Some(why) = self.spent() {
            return Err(Err(MindError::Declined(why)));
        }
        state.asked += 1;
        Ok(state.prepare(request, &self.settings))
    }

    /// Whether the endpoint answers: the model's entry, or the model list.
    async fn probe(&self) -> bool {
        let url = match self.settings.provider {
            Provider::Anthropic => {
                anthropic::model_url(&self.credentials.base, &self.settings.model)
            }
            Provider::OpenAi => openai::models_url(&self.credentials.base),
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
fn ask_again(
    provider: Provider,
    messages: &mut Vec<Value>,
    reply: &Reply,
    why: &str,
    question: u64,
) {
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
    match provider {
        Provider::Anthropic => anthropic::nudge(messages, &results, &text),
        Provider::OpenAi => openai::user(messages, &results, None, &text),
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
    provider: Provider,
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
        let why = match provider {
            Provider::Anthropic => anthropic::error_message(&body),
            Provider::OpenAi => openai::error_message(&body),
        }
        .unwrap_or_else(|| "no reason given".into());
        return Err(Failed {
            turned_down: matches!(status, 400 | 413),
            ..Failed::unbilled(format!("{status}: {why}"))
        });
    }
    Ok(match provider {
        Provider::Anthropic => anthropic::parse(&body),
        Provider::OpenAi => openai::parse(&body, mode),
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
            (AnswerMode::Json, _) => "answer with one JSON object and nothing else".into(),
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
    /// What to tell the model when the cast it last answered did not
    /// happen. The table takes back a cast whose whole cost cannot be paid
    /// and gives priority back (CR 601.2h, 732.1, 732.2) without a word, so
    /// the model would see the same question again and may answer it the
    /// same way, again.
    ///
    /// A cast that happened is in the log ([`LogEvent::Cast`]), which the
    /// mind is handed whole, a line at a time: from the cast's answer on,
    /// every request's lines are read for it. One that did not reach the
    /// log by the next priority, with its card still in the hand or the
    /// command zone, was taken back. The card's place alone would not say
    /// so: an object keeps its handle across zones, so a spell that
    /// resolved and came back to the hand stands where it was cast from.
    /// A refused answer says why itself, and gets no second reason.
    fn undone(&mut self, request: &Request) -> Option<String> {
        let card = self.casting?;
        let cast = request.log.entries.iter().any(|entry| {
            matches!(
                &entry.event,
                LogEvent::Cast { spell: LogObject::Known { id, .. }, .. } if *id == card
            )
        });
        if cast {
            self.casting = None;
            return None;
        }
        if !matches!(request.pending, Pending::Priority { .. }) {
            return None;
        }
        self.casting = None;
        if request.retry.is_some() {
            return None;
        }
        let view = &request.view;
        let name = view
            .hand
            .iter()
            .find(|c| c.id == card)
            .map(|c| c.name.clone())
            .or_else(|| {
                view.command
                    .iter()
                    .flatten()
                    .find(|o| o.id == card)
                    .map(|o| o.name.clone())
            })?;
        Some(format!(
            "Your cast of {name} {} did not happen: the card is where it was, and you have \
             priority again. The table takes back a cast whose whole cost cannot be paid, \
             with any kicker or additional cost you said yes to (CR 601.2h, 732.1); count \
             the cost before you cast it again.",
            narrator::tag(card)
        ))
    }

    /// The table or the referee refused the seat's last answer.
    fn refused(&mut self, reason: &str, action: &PlayerAction, mode: AnswerMode) {
        self.plan = None;
        self.hint = None;
        let said = format!("Your answer was refused: {reason}. Answer the same question again.");
        if self.last_by_model
            && mode == AnswerMode::Tools
            && let Some(result) = self.results.first_mut()
        {
            result.content = said;
            result.is_error = true;
            return;
        }
        if self.last_by_model {
            self.notes.push(said);
        } else {
            self.notes.push(format!(
                "Your plan stopped: the table refused {action:?} ({reason})."
            ));
        }
    }

    /// The answer the plan or the hint gives, when either fits.
    fn follow(&mut self, request: &Request) -> Option<(PlayerAction, String)> {
        if let Some(action) = self.follow_plan(request) {
            let label = self
                .plan
                .as_ref()
                .map_or_else(|| "the plan's last step".to_string(), |p| p.label.clone());
            return Some((action, label));
        }
        if self.plan.is_none() && matches!(request.pending, Pending::Priority { .. }) {
            self.hint = None;
        }
        self.follow_hint(request)
            .map(|action| (action, "the targets named ahead".to_string()))
    }

    fn follow_plan(&mut self, request: &Request) -> Option<PlayerAction> {
        let plan = self.plan.as_mut()?;
        let view = &request.view;
        let same = (view.turn, view.phase, view.step) == plan.at;
        match &request.pending {
            Pending::ChooseColor { options, .. } if same => {
                if let Some(color) = plan.color.take().filter(|c| options.contains(c)) {
                    return Some(PlayerAction::ChooseColor(color));
                }
            }
            Pending::Priority { legal, .. } if same => {
                if let Some(step) = plan.steps.front().copied() {
                    let action = narrator::tap(&step);
                    if offered(legal, &action) {
                        plan.steps.pop_front();
                        plan.color = step.color;
                        return Some(action);
                    }
                } else if offered(legal, &plan.then) {
                    let then = plan.then.clone();
                    self.plan = None;
                    return Some(then);
                }
            }
            _ => {}
        }
        let plan = self.plan.take()?;
        self.notes.push(format!(
            "Your plan ({}) stopped before it finished: the table asked something else. Any \
             mana it made is in your pool.",
            plan.label
        ));
        None
    }

    fn follow_hint(&mut self, request: &Request) -> Option<PlayerAction> {
        let hint = self.hint.take()?;
        let Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            reason: TargetPrompt::Targets,
            ..
        } = &request.pending
        else {
            // A cast mode, an X or a colour may come before the targets.
            if !matches!(request.pending, Pending::Priority { .. }) {
                self.hint = Some(hint);
            }
            return None;
        };
        let targeting = request.view.targeting.as_ref()?;
        let card = targeting
            .source
            .rules
            .map(|r| r.card)
            .or_else(|| targeting.source.card.map(|c| c.index));
        let count = hint.objects.len() + hint.players.len();
        let fits = card == Some(hint.card)
            && !targeting.second
            && (usize::from(*min)..=usize::from(*max)).contains(&count)
            && hint.objects.iter().all(|o| options.contains(o))
            && hint.players.iter().all(|p| player_options.contains(p));
        if !fits {
            self.notes
                .push("The targets you named ahead are not legal now: choose them here.".into());
            return None;
        }
        Some(PlayerAction::ChooseTargets {
            objects: hint.objects,
            players: hint.players,
        })
    }

    /// Tells the decision and builds the conversation that asks it.
    fn prepare(&mut self, request: &Request, settings: &Settings) -> Prepared {
        let long = || {
            serde_json::to_string(&self.messages).map_or(0, |text| narrator::estimate_tokens(&text))
                > settings.conversation_tokens
        };
        let fresh = self.messages.is_empty() || request.view.turn != self.turn || long();
        let mut told = Vec::new();
        if fresh && !self.says.is_empty() {
            let said: Vec<String> = self.says.iter().map(|s| format!("  - {s}")).collect();
            told.push(format!(
                "Your notes from earlier turns:\n{}",
                said.join("\n")
            ));
        }
        // Kept until the model answers: a message it never read is told
        // again.
        told.extend(self.notes.iter().cloned());
        let mut narrator = self.narrator.clone();
        if fresh {
            narrator.forget_cards();
        }
        let wake = narrator.wake(request, &told);
        let prefix = fresh.then(|| narrator::prefix(&request.context));
        let (mut messages, results) = if fresh {
            let start = match settings.provider {
                Provider::Anthropic => Vec::new(),
                Provider::OpenAi => vec![openai::system(settings.answer)],
            };
            (start, Vec::new())
        } else {
            (self.messages.clone(), self.results.clone())
        };
        match settings.provider {
            Provider::Anthropic => {
                anthropic::user(&mut messages, &results, prefix.as_deref(), &wake.text);
            }
            Provider::OpenAi => {
                openai::user(&mut messages, &results, prefix.as_deref(), &wake.text);
            }
        }
        Prepared {
            wake,
            narrator,
            messages,
            asked: self.asked,
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
        self.narrator = narrator;
        self.turn = request.view.turn;
        self.notes.clear();
        self.last_by_model = true;
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
        if let Some(say) = &read.say {
            self.says.push_back(say.chars().take(300).collect());
            while self.says.len() > SAYS {
                self.says.pop_front();
            }
        }
        self.hint.clone_from(&read.resolved.hint);
        let action = match read.resolved.act {
            Act::Now(action) => {
                self.plan = None;
                action
            }
            Act::Taps { steps, then } => {
                let mut steps: VecDeque<manaplan::Step> = steps.into();
                let first = steps.pop_front();
                let view = &request.view;
                self.plan = Some(Plan {
                    steps,
                    then: then.clone(),
                    label: label.clone(),
                    at: (view.turn, view.phase, view.step),
                    color: first.and_then(|s| s.color),
                });
                if let Some(step) = first {
                    narrator::tap(&step)
                } else {
                    self.plan = None;
                    then
                }
            }
        };
        let note = json!({
            "chose": label,
            "say": read.say,
            "reasoning": reply.reasoning,
            "tokens": reply.usage,
            "ms": u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        });
        Answer {
            action,
            model_time: took,
            note: Some(note.to_string()),
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
        self.transcript.write_value(&json!({
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
        self.transcript.flush();
    }
}

/// Whether `legal` offers `action` now.
fn offered(legal: &LegalActions, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::ActivateManaAbility { source } => legal.mana_abilities.contains(source),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => legal.abilities.contains(&(*source, *ability_index)),
        PlayerAction::CastSpell { card } => legal.castable.contains(card),
        PlayerAction::PassPriority => legal.can_pass,
        _ => false,
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
                lock(&seat).casting = Some(*card);
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
