//! Echoes revision two. Existing revision-one runs retain their original rules.
//!
//! These pure transitions require an authenticated authority, durable revision
//! lock and request deduplication at the service boundary; they grant no access.
pub mod catalog;
pub mod economy;
pub mod execution;
pub mod generation;
pub mod inventory;
pub mod loadout;
pub mod matching;
pub mod package;
pub mod run;
pub mod shop;
pub mod verification;

/// Native setup namespace reserved for Tower.
pub const PACK: u16 = 1;
/// Immutable native setup and economy revision.
pub const REVISION: u16 = generation::CURRENT.revision;
/// Frozen offer-generation algorithm for this native revision.
pub const GENERATOR: u16 = generation::RUN;
/// Run-local starting currency.
pub const STARTING_GOLD: u16 = 18;
/// Equal reward for a verified win, loss or rules draw.
pub const DUEL_INCOME: u16 = 12;
/// Victories required to conquer this expedition.
pub const TARGET_WINS: u8 = 8;
/// Unsuccessful duels before exhaustion.
pub const DEFEAT_LIMIT: u8 = 5;
/// Maximum duels in one run, counting losses and rules draws.
pub const MAX_STAGES: u8 = TARGET_WINS + DEFEAT_LIMIT - 1;
