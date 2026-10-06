//! A table's language-model chairs, as the host's client seats them
//! (`docs/llm-seat.md` §"A language model at your table").
//!
//! An AI chair at a gateway table is the house, run by the engine; a chair
//! played by a language model is an open chair that a seat bridge
//! (`baylee-seat join`) started by the host's client sits down in, as any
//! player would. This module decides everything about that and touches
//! nothing: which profile, model and effort a chair plays ([`ChairModel`]),
//! the bridge's command line ([`bridge_args`]), which bridges to start and
//! stop as the room changes ([`Seating::steps`]), and, in a debug build,
//! what a running bridge is told when its chair is changed during the game
//! ([`Order`], [`Seating::change`]). The shell spawns the processes and
//! writes the orders; the bridge reads an order with [`Order::parse`].
//!
//! No key goes anywhere from here: a bridge reads its key from the variable
//! its profile names, else from this machine's credential store
//! (`super::keys`), and neither a command line nor an order carries one
//! ([`Order::parse`] refuses a value shaped like one).

use super::models::{Endpoint, Resolved, offered, resolve};
use super::{
    AnswerMode, CliTool, DEFAULT_ANTHROPIC_MODEL, Profile, Provider, SeatSettings,
    effort_is_a_word, model_fault, profile_name_is_a_name, shaped_like_a_key,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whether this build lets a chair be changed while its game is on: a
/// debug build only. A release bridge reads no orders, and a release
/// client offers none.
pub const LIVE_CHANGES: bool = cfg!(debug_assertions);

/// What a language-model chair plays: a profile of the settings file, and
/// the model and effort over it. The profile gives everything else (the
/// provider and its address or program, the key's variable, the limits).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChairModel {
    /// The profile's name.
    pub profile: String,
    /// The model's exact id, as `--mind` carries it.
    pub model: String,
    /// The effort named, or `None` for the build's default (medium on
    /// Anthropic, the model's own elsewhere), whatever the profile names.
    pub effort: Option<String>,
}

impl ChairModel {
    /// The chair playing `profile` as the file has it.
    #[must_use]
    pub fn of(name: &str, profile: &Profile) -> Self {
        Self {
            profile: name.to_string(),
            model: profile.model.clone(),
            effort: profile.effort.clone(),
        }
    }

    /// Plays `model` instead, keeping the effort only where the new model
    /// takes it ([`Resolved::takes`]).
    pub fn choose_model(&mut self, model: &Resolved) {
        self.model.clone_from(&model.id);
        if self.effort.as_deref().is_some_and(|e| !model.takes(e)) {
            self.effort = None;
        }
    }

    /// Names `effort` (`None`: the model's own). Refused (`false`, nothing
    /// changed) for an effort the model does not take.
    pub fn choose_effort(&mut self, model: &Resolved, effort: Option<&str>) -> bool {
        if effort.is_some_and(|e| !model.takes(e)) {
            return false;
        }
        self.effort = effort.map(str::to_string);
        true
    }

    /// Writes its model and effort into `profile`: the chair's choice, kept
    /// as the profile's.
    pub fn save_into(&self, profile: &mut Profile) {
        profile.model.clone_from(&self.model);
        profile.effort.clone_from(&self.effort);
    }
}

/// The endpoint `profile` plays through, with `env_base` the address its
/// provider's variable names on this machine (`BAYLEE_LLM_BASE_URL`, read by
/// the shell), for an API profile that names none.
#[must_use]
pub fn endpoint<'a>(profile: &'a Profile, env_base: Option<&'a str>) -> Endpoint<'a> {
    match profile.provider {
        Provider::Cli => Endpoint::cli(&profile.model),
        provider => Endpoint::new(provider, profile.base_url.as_deref().or(env_base)),
    }
}

/// The models a chair playing `profile` is offered: [`offered`], and the
/// profile's own model wherever it is not among them, so the chair always
/// shows what it plays.
#[must_use]
pub fn models_for(profile: &Profile, env_base: Option<&str>, listed: &[String]) -> Vec<Resolved> {
    let at = endpoint(profile, env_base);
    let mut out = offered(at, listed);
    if !out.iter().any(|m| m.id == profile.model) {
        out.push(resolve(at, &profile.model, listed));
    }
    out
}

/// `model` as a chair playing `profile` shows it.
#[must_use]
pub fn resolved(
    profile: &Profile,
    env_base: Option<&str>,
    model: &str,
    listed: &[String],
) -> Resolved {
    resolve(endpoint(profile, env_base), model, listed)
}

/// What starts a bridge for a chair, beside its [`ChairModel`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch<'a> {
    /// The room's id.
    pub room: &'a str,
    /// The gateway's address, as the client reached it.
    pub gateway: &'a str,
    /// The chair, numbered as the gateway numbers it (from 0).
    pub chair: u32,
    /// The settings file the client read the profile from.
    pub config: &'a str,
    /// The acceptance deck it brings.
    pub deck: &'a str,
    /// The house level that answers when the model cannot.
    pub level: &'a str,
}

/// The acceptance decks a chair may bring (`data/acceptance-decks.txt`),
/// the first being the bridge's own default.
pub const DECKS: [&str; 4] = ["Allytifact", "Schwarzrand", "Euro-Highlander", "Weltenbaum"];

/// The provider's name in `--mind`.
fn mind(provider: Provider, model: &str) -> String {
    format!("{}:{model}", provider.name())
}

/// The bridge's arguments, after its program: `join` the room in the
/// chair, playing the profile under the chair's model and effort, with the
/// settings file named (so the bridge reads the file the client showed),
/// held to the client by its stdin (`--tethered`). A model or effort the
/// profile already says is not repeated. No key, ever: the profile names
/// the variable.
#[must_use]
pub fn bridge_args(launch: &Launch<'_>, chair: &ChairModel, profile: &Profile) -> Vec<String> {
    let mut args: Vec<String> = [
        "join",
        launch.room,
        "--gateway",
        launch.gateway,
        "--chair",
        &launch.chair.to_string(),
        "--config",
        launch.config,
        "--profile",
        &chair.profile,
        "--acceptance",
        launch.deck,
        "--level",
        launch.level,
        "--tethered",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    if chair.model != profile.model {
        args.extend(["--mind".into(), mind(profile.provider, &chair.model)]);
    }
    if chair.effort != profile.effort {
        match &chair.effort {
            Some(effort) => args.extend(["--effort".into(), effort.clone()]),
            // The profile names one and the chair plays the default: the
            // profile's must not stand (the model may not take it).
            None => args.push("--default-effort".into()),
        }
    }
    args
}

/// What a debug bridge is told when its chair changes during the game: one
/// JSON line on its stdin. It takes effect at the bridge's next decision; a
/// question already being thought about is answered by the mind that was
/// asked it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Order {
    /// The house plays the chair, at `level`.
    House {
        /// The house's level (`steady`, `sharp`, …).
        level: String,
    },
    /// A language model plays it: a profile of the bridge's settings file,
    /// its model and effort over it.
    Model {
        /// The profile.
        profile: String,
        /// The model's exact id.
        model: String,
        /// The effort, or `None` for the build's default, whatever the
        /// profile names.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effort: Option<String>,
    },
}

/// The longest order a bridge reads.
pub const ORDER_BYTES: usize = 1024;

impl Order {
    /// The order to play `chair`.
    #[must_use]
    pub fn model(chair: &ChairModel) -> Self {
        Self::Model {
            profile: chair.profile.clone(),
            model: chair.model.clone(),
            effort: chair.effort.clone(),
        }
    }

    /// The line the bridge reads: JSON, and a newline.
    #[must_use]
    pub fn line(&self) -> String {
        let mut line = serde_json::to_string(self).unwrap_or_default();
        line.push('\n');
        line
    }

    /// Reads one line.
    ///
    /// # Errors
    /// A sentence, for a line that is too long, not an order, or that holds
    /// a value shaped like a key (said without it), a profile name, model
    /// id, effort or level that is not one.
    pub fn parse(line: &str) -> Result<Self, String> {
        if line.len() > ORDER_BYTES {
            return Err(format!(
                "an order is one line of at most {ORDER_BYTES} bytes"
            ));
        }
        if shaped_like_a_key(line) {
            return Err("an order holds no key: a profile names its key's variable".into());
        }
        let order: Self =
            serde_json::from_str(line.trim()).map_err(|e| format!("not an order: {e}"))?;
        match &order {
            Self::House { level } if !effort_is_a_word(level) => {
                Err("a house level is a word such as steady".into())
            }
            Self::Model {
                profile,
                model,
                effort,
            } => {
                if !profile_name_is_a_name(profile) {
                    return Err("an order names a profile of the settings file".into());
                }
                let own = model.split_once(':').map_or(model.as_str(), |(_, own)| own);
                if let Some(why) = model_fault(own) {
                    return Err(why);
                }
                if effort.as_deref().is_some_and(|e| !effort_is_a_word(e)) {
                    return Err("an effort is a word such as low, medium or high".into());
                }
                Ok(order)
            }
            Self::House { .. } => Ok(order),
        }
    }

    /// `--mind`'s spelling of its model over `provider`'s profile
    /// (`anthropic:claude-opus-5-5`, `cli:claude:opus`); `None` for the
    /// house.
    #[must_use]
    pub fn mind(&self, provider: Provider) -> Option<String> {
        match self {
            Self::House { .. } => None,
            Self::Model { model, .. } => Some(mind(provider, model)),
        }
    }
}

/// Where the room is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for players: a chair is seated by starting a bridge.
    Waiting,
    /// The game is on: a bridge holds its chair to the end.
    Playing,
}

/// What a chair's change comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// Before the game: the chair's bridge is stopped (it leaves the chair)
    /// and one with the new choice started ([`Seating::steps`]).
    Reseat,
    /// During the game, in a debug build: this order to the running bridge.
    Order(Order),
}

/// Why a change was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The game is on and this is a release build, whose bridge reads no
    /// orders.
    NotLive,
    /// No language model was seated in that chair by this client.
    NotOurs,
}

/// A chair this client seats, and how many times its choice changed: a
/// bridge started for an older choice is stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    /// What it plays.
    pub model: ChairModel,
    /// The deck it brings.
    pub deck: String,
    /// Bumped by every change before the game.
    pub version: u64,
}

/// A bridge the shell started, as it reports it to [`Seating::steps`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Running {
    /// Its chair.
    pub chair: u32,
    /// The [`Planned::version`] it was started for.
    pub version: u64,
    /// Whether it has exited (refused, or its game is over). An exited one
    /// is not started again for the same version: its last line says why.
    pub exited: bool,
}

/// What the shell does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Start a bridge for this chair, at its planned version.
    Launch(u32),
    /// Stop this chair's bridge, and forget it.
    Stop(u32),
}

/// The language-model chairs of the room this client hosts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seating {
    room: Option<String>,
    chairs: BTreeMap<u32, Planned>,
}

impl Seating {
    /// The room it seats.
    #[must_use]
    pub fn room(&self) -> Option<&str> {
        self.room.as_deref()
    }

    /// Seats in `room` from now on; a different room forgets every chair
    /// of the last (whose bridges [`Self::steps`] then stops).
    pub fn enter(&mut self, room: &str) {
        if self.room.as_deref() != Some(room) {
            self.room = Some(room.to_string());
            self.chairs.clear();
        }
    }

    /// Forgets the room: every bridge is to stop.
    pub fn leave(&mut self) {
        self.room = None;
        self.chairs.clear();
    }

    /// The chair's plan, if a language model is to play it.
    #[must_use]
    pub fn chair(&self, chair: u32) -> Option<&Planned> {
        self.chairs.get(&chair)
    }

    /// Every planned chair.
    pub fn chairs(&self) -> impl Iterator<Item = (u32, &Planned)> {
        self.chairs.iter().map(|(at, planned)| (*at, planned))
    }

    /// Plans `model` for `chair` before the game, with `deck` (the one it
    /// had, or the bridge's default for a new chair).
    pub fn plan(&mut self, chair: u32, model: ChairModel, deck: Option<&str>) -> Change {
        let version = self.chairs.get(&chair).map_or(0, |p| p.version + 1);
        let deck = deck
            .map(str::to_string)
            .or_else(|| self.chairs.get(&chair).map(|p| p.deck.clone()))
            .unwrap_or_else(|| DECKS[0].to_string());
        self.chairs.insert(
            chair,
            Planned {
                model,
                deck,
                version,
            },
        );
        Change::Reseat
    }

    /// No language model plays `chair` any more (the host made it open or
    /// the house's): its bridge stops before the game.
    pub fn unplan(&mut self, chair: u32) {
        self.chairs.remove(&chair);
    }

    /// Changes what a seated chair plays. Before the game the plan changes
    /// and the bridge is started again ([`Change::Reseat`]); during it,
    /// only where `live` (a debug build, [`LIVE_CHANGES`]), the running
    /// bridge is told ([`Change::Order`]) and the plan shows the new
    /// choice. `None` hands the chair to the house at `level`, which only
    /// an order can do: before the game the house's chair is the
    /// gateway's, and the shell makes it one.
    ///
    /// # Errors
    /// [`Refusal::NotLive`] during a release build's game, and
    /// [`Refusal::NotOurs`] for a chair no bridge of this client plays.
    pub fn change(
        &mut self,
        chair: u32,
        to: Option<ChairModel>,
        level: &str,
        phase: Phase,
        live: bool,
    ) -> Result<Change, Refusal> {
        if !self.chairs.contains_key(&chair) {
            return Err(Refusal::NotOurs);
        }
        match (phase, to) {
            (Phase::Waiting, Some(model)) => Ok(self.plan(chair, model, None)),
            (Phase::Waiting, None) => {
                self.unplan(chair);
                Ok(Change::Reseat)
            }
            (Phase::Playing, _) if !live => Err(Refusal::NotLive),
            (Phase::Playing, Some(model)) => {
                let order = Order::model(&model);
                if let Some(planned) = self.chairs.get_mut(&chair) {
                    planned.model = model;
                }
                Ok(Change::Order(order))
            }
            (Phase::Playing, None) => Ok(Change::Order(Order::House {
                level: level.to_string(),
            })),
        }
    }

    /// What to start and stop. Before the game (`phase` waiting): a bridge
    /// for a chair no longer planned, or for an older version of its plan,
    /// is stopped; a planned chair with no bridge for its version is
    /// started once the room lists it `open` (no one sitting there, the
    /// stopped bridge gone too). During
    /// the game nothing is started or stopped: a bridge holds its chair to
    /// the end, and only the leaving of the room ([`Self::leave`]) stops it.
    #[must_use]
    pub fn steps(&self, phase: Phase, open: &[u32], running: &[Running]) -> Vec<Step> {
        let mut out = Vec::new();
        let gone = |r: &Running| {
            self.chairs
                .get(&r.chair)
                .is_none_or(|planned| planned.version != r.version)
        };
        for r in running {
            if self.room.is_none() || (phase == Phase::Waiting && gone(r)) {
                out.push(Step::Stop(r.chair));
            }
        }
        if phase == Phase::Waiting && self.room.is_some() {
            for (chair, planned) in &self.chairs {
                // A chair whose old bridge is stopping is still taken until
                // that bridge has left it: the new one starts on a later
                // look, once the room lists the chair open.
                let started = running
                    .iter()
                    .any(|r| r.chair == *chair && r.version == planned.version);
                if !started && open.contains(chair) {
                    out.push(Step::Launch(*chair));
                }
            }
        }
        out
    }
}

/// What a player reads for a wire protocol: an adapter is a protocol and
/// the address it is spoken to, not a vendor.
#[must_use]
pub const fn protocol_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "Anthropic Messages",
        Provider::OpenAi => "OpenAI-compatible",
        Provider::Cli => "Agent CLI",
    }
}

/// A ready-made adapter: a wire protocol and the address it is spoken to,
/// with the variable its key is read from, made into a profile of the
/// settings file in one press ([`Preset::profile`]). The player may change
/// the address and the model afterwards; the preset only fills them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// Anthropic Messages at Anthropic, its key in `ANTHROPIC_API_KEY`.
    Anthropic,
    /// OpenAI-compatible at `OpenAI`, its key in `OPENAI_API_KEY`.
    OpenAi,
    /// OpenAI-compatible at `DeepSeek`, its key in `DEEPSEEK_API_KEY`.
    DeepSeek,
    /// Anthropic Messages at `DeepSeek`'s Anthropic-compatible address, the
    /// same key as [`Preset::DeepSeek`].
    DeepSeekAnthropic,
    /// OpenAI-compatible at LM Studio's address on this machine: no key
    /// needed, its models listed.
    LmStudio,
    /// Claude Code, on its own login.
    ClaudeCode,
    /// Gemini CLI (Antigravity), on its own login.
    Agy,
}

impl Preset {
    /// Every one, in the order offered.
    pub const ALL: [Self; 7] = [
        Self::Anthropic,
        Self::OpenAi,
        Self::DeepSeek,
        Self::DeepSeekAnthropic,
        Self::LmStudio,
        Self::ClaudeCode,
        Self::Agy,
    ];

    /// What a player reads: whose address, and which protocol.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
            Self::DeepSeek => "DeepSeek",
            Self::DeepSeekAnthropic => "DeepSeek (Anthropic protocol)",
            Self::LmStudio => "LM Studio (this machine)",
            Self::ClaudeCode => "Claude Code",
            Self::Agy => "Antigravity (agy)",
        }
    }

    /// The name its profile is given, or the first free one after it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::DeepSeek => "deepseek",
            Self::DeepSeekAnthropic => "deepseek-anthropic",
            Self::LmStudio => "lmstudio",
            Self::ClaudeCode => "claude-code",
            Self::Agy => "agy",
        }
    }

    /// Its profile: the protocol, the address and the key's variable, the
    /// build's defaults otherwise, and what each needs to sit down (a token
    /// budget for a model with no price, the answer a server without tools
    /// gives). Each vendor's key has its own variable, so two adapters
    /// never share a key by accident.
    #[must_use]
    pub fn profile(self) -> Profile {
        let deepseek = |provider, base: &str| Profile {
            base_url: Some(base.into()),
            key_env: Some("DEEPSEEK_API_KEY".into()),
            game_tokens: Some(3_000_000),
            ..Profile::new(provider, "deepseek-flash")
        };
        match self {
            Self::Anthropic => Profile::new(Provider::Anthropic, DEFAULT_ANTHROPIC_MODEL),
            Self::OpenAi => Profile {
                base_url: Some("https://api.openai.com/v1".into()),
                key_env: Some("OPENAI_API_KEY".into()),
                game_tokens: Some(super::DEFAULT_SPEND_TOKENS),
                ..Profile::new(Provider::OpenAi, "gpt-5")
            },
            Self::DeepSeek => Profile {
                answer: Some(AnswerMode::Json),
                ..deepseek(Provider::OpenAi, "https://api.deepseek.com/v1")
            },
            Self::DeepSeekAnthropic => {
                deepseek(Provider::Anthropic, "https://api.deepseek.com/anthropic")
            }
            Self::LmStudio => Profile {
                base_url: Some("http://127.0.0.1:1234/v1".into()),
                answer: Some(AnswerMode::JsonSchema),
                game_tokens: Some(super::DEFAULT_SPEND_TOKENS),
                ..Profile::new(Provider::OpenAi, "local-model")
            },
            Self::ClaudeCode => Profile::new(Provider::Cli, "claude:opus"),
            Self::Agy => Profile::new(Provider::Cli, CliTool::Agy.name()),
        }
    }

    /// Adds its profile to `settings` under its name or the first free one
    /// after it (`anthropic-2`); the name.
    pub fn add_to(self, settings: &mut SeatSettings) -> String {
        let base = self.name();
        // A file holds at most as many profiles as it has, so one of the
        // first `len + 1` names after the plain one is free.
        let most = settings.profiles.len() + 2;
        let name = std::iter::once(base.to_string())
            .chain((2..=most).map(|n| format!("{base}-{n}")))
            .find(|name| !settings.profiles.contains_key(name))
            .unwrap_or_else(|| base.to_string());
        settings.profiles.insert(name.clone(), self.profile());
        name
    }
}

/// Points `profile` at `base` (an adapter's address the player typed),
/// trimmed and without a trailing slash; `None` puts it back to the
/// protocol's own. A CLI has no address.
///
/// # Errors
/// A sentence, for a CLI or an address that would carry a key in the clear
/// ([`super::address_fault`]); the profile is then unchanged.
pub fn set_address(profile: &mut Profile, base: Option<&str>) -> Result<(), String> {
    if profile.provider == Provider::Cli {
        return Err("a CLI is a program on this machine and has no address".into());
    }
    let base = base
        .map(|b| b.trim().trim_end_matches('/').to_string())
        .filter(|b| !b.is_empty());
    if let Some(base) = &base {
        if let Some(why) = super::address_fault(base, "the address") {
            return Err(why);
        }
        if super::shaped_like_a_key(base) {
            return Err(
                "the address holds what looks like a key: keys never go in the file".into(),
            );
        }
    }
    profile.base_url = base;
    Ok(())
}

#[cfg(test)]
#[path = "seating_tests.rs"]
mod tests;
