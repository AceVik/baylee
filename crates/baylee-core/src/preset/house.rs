use super::{HandSharingRule, LoopPolicy, StartingCounters};
use serde::{Deserialize, Serialize};

/// Per-game house rules (versioned with the preset).
///
/// House rules genuinely are a bag of independent toggles.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct HouseRules {
    /// First mulligan is free (default).
    pub mulligan_free_first: bool,
    /// Explicit house-rule count; absent preserves the legacy first-free flag.
    #[serde(default)]
    pub free_mulligans: Option<u8>,
    /// Counters seeded on configured battlefield cards, before play.
    #[serde(default)]
    pub starting_counters: Vec<StartingCounters>,
    /// Endless-loop policy.
    pub loop_policy: LoopPolicy,
    /// Per-decision timeout in seconds, one allowance for every question a
    /// seat is asked — priority, attackers, a discard, an opening hand
    /// alike. Default 180, three minutes (owner, 08.10.2026; it was 600).
    /// `0` = no decision clock.
    pub decision_timeout_secs: u32,
    /// Reconnect window before AI takes over a seat (default 60 s).
    pub reconnect_window_secs: u32,
    /// Optional explicit starting seat for controlled games. `None` chooses
    /// uniformly from the preset's seeded RNG. Missing on an old record keeps
    /// the historical seat 0; new random starts serialize as `null`.
    #[serde(default = "legacy_starting_player")]
    pub starting_player: Option<u8>,
    /// Remove an optional trigger when an empty target answer is proved to
    /// have no effect and no follow-up question. New games enable this; a
    /// record without the field keeps its historical target questions.
    #[serde(default)]
    pub auto_discard_empty_optional_triggers: bool,
    /// Anti-tell: auto-passes fire with normalized random delay.
    pub timing_normalization: bool,
    /// Allow opponent-approved takebacks.
    pub takebacks: bool,
    /// Allow players to vote on time extensions (AI always accepts).
    pub time_extension_votes: bool,
    /// Ask a player whose triggers differ what order they go on the stack
    /// in (CR 603.3b); without it the timestamps decide.
    ///
    /// Off unless a table says so, which real tables do
    /// ([`Self::for_a_table`]), and off in a training or evaluation preset,
    /// so that the net's numbers and records stay what they are. A record
    /// without the field keeps its timestamps, and writing a game with the
    /// rule off writes exactly what it wrote before the field existed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub order_triggers: bool,
    /// Ask the controller of an attacker with trample blocked by two or more
    /// creatures how its damage is divided (CR 702.19b); without it each
    /// blocker takes lethal damage in declaration order and the rest goes
    /// past them.
    ///
    /// Set and stored exactly as [`Self::order_triggers`], for the same
    /// reason: only a table asks, and a game with the rule off writes and
    /// hashes what it did before the field existed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub divide_trample: bool,
    /// Whether and how teammates may voluntarily share their hands (#265).
    ///
    /// Defaults to [`HandSharingRule::Allowed`] and is omitted on the wire and
    /// in records when at its default, so existing presets and hashes stay
    /// byte-identical.
    #[serde(default, skip_serializing_if = "HandSharingRule::is_default")]
    pub hand_sharing: HandSharingRule,
    /// Private, frozen opponent-policy seed for an official challenge. Independent
    /// of the shuffle seed and every attempt's public game ID. Its presence also
    /// requires untimed play without stand-ins, takebacks or adaptive house search.
    /// Absent on ordinary presets and old records, preserving their wire bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_policy_seed: Option<u64>,
}

/// Seconds a seat has to answer one question unless the table says
/// otherwise: three minutes (owner, 08.10.2026). The gateway's default
/// clock (`clock::PRESETS[0]`) is the same number, and a test there holds
/// the two together.
pub const DEFAULT_DECISION_SECS: u32 = 180;

#[allow(clippy::unnecessary_wraps)] // Serde's field default must match the optional field.
fn legacy_starting_player() -> Option<u8> {
    Some(0)
}

impl Default for HouseRules {
    fn default() -> Self {
        Self {
            mulligan_free_first: true,
            free_mulligans: None,
            starting_counters: Vec::new(),
            loop_policy: LoopPolicy::default(),
            decision_timeout_secs: DEFAULT_DECISION_SECS,
            reconnect_window_secs: 60,
            starting_player: None,
            auto_discard_empty_optional_triggers: true,
            timing_normalization: true,
            takebacks: false,
            time_extension_votes: true,
            order_triggers: false,
            divide_trample: false,
            hand_sharing: HandSharingRule::default(),
            challenge_policy_seed: None,
        }
    }
}

impl HouseRules {
    /// The rules of a game played at a table, by people or language models:
    /// the defaults, with the questions [`Self::default`] leaves out of a
    /// training or evaluation game because the net cannot be asked them.
    #[must_use]
    pub fn for_a_table() -> Self {
        Self {
            order_triggers: true,
            divide_trample: true,
            ..Self::default()
        }
    }

    /// Number of opening redraws that cost no cards.
    #[must_use]
    pub fn free_mulligan_count(&self) -> u8 {
        self.free_mulligans
            .unwrap_or(u8::from(self.mulligan_free_first))
    }
}
