//! Global expedition domain. No account service, database, clock, network or game execution.
//!
//! Commands are applied by the Tower authority under its durable run lock. Public
//! projections do not grant permission to mutate them; clients send commands only.
#![forbid(unsafe_code)]

pub mod cards;
pub mod economy;
pub mod items;
pub mod loadout;
pub mod matching;
pub mod package;
pub mod run;
pub mod shop;

/// Initial, unpublished balance revision. Published revisions are immutable.
pub const RULES_VERSION: u32 = 1;
/// A run starts with enough gold for an ordinary improvement and a consumable.
pub const STARTING_GOLD: u16 = 12;
/// Equal base income after a verified win or loss, avoiding winner-income snowballing.
pub const DUEL_INCOME: u16 = 10;
/// Hard bound on a run's wallet, independent of incoming serialized data.
pub const MAX_GOLD: u16 = 200;
/// Five victories complete the initial expedition.
pub const TARGET_WINS: u8 = 5;
/// Three unsuccessful duels end the run; independent of life at the table.
pub const DEFEAT_LIMIT: u8 = 3;
/// Maximum number of duels before victory or exhaustion.
pub const MAX_STAGES: u8 = TARGET_WINS + DEFEAT_LIMIT - 1;

/// A refused domain operation; never embeds a private deck, account or seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// Stored or incoming state does not satisfy this rules revision.
    #[error("invalid Tower state")]
    Invalid,
    /// The expected revision or shop offer no longer matches.
    #[error("Tower state changed")]
    Stale,
    /// The wallet cannot pay the authoritative price.
    #[error("not enough gold")]
    Gold,
    /// A purchase or loadout exceeds an inventory/slot/rank bound.
    #[error("Tower inventory limit")]
    Capacity,
    /// The item is not owned or a card is outside the run's collection.
    #[error("Tower item unavailable")]
    Unavailable,
}
