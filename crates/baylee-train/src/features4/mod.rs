//! Playernet v4 state, observation-stream, relational-graph, and
//! candidate/action encoder (`ENCODER_VERSION = 5`).
//!
//! Implements the v4 representation described in
//! `docs/history/trained-ai-v4-handover-2026-10-22.md`, with remaining
//! representation limits tracked in the Codex takeover log:
//!
//! - **Full 8-seat relative coordinates** (`0..7`, `8 = none`) across every
//!   entity, seat, combat arrow, commander-damage tally, and candidate.
//! - **Expanded `PublicObject`, `SeatView`, and `PlayerView` features**:
//!   all 7 `ObjectStatus` bits, all 6 `offered` bits + numeric `picked`,
//!   active face (`card.face`), `rules` (`RulesFace`), `prepared_spell`,
//!   `chosen_subtype`, `chosen_name`, `chosen_opponent`, `unlocked_doors`,
//!   `suspended`, `battle_protector`, all 21 `CounterKind` categories,
//!   `granted_mana`, `board_mana`, `flashback`, `grants`, `stack_item`,
//!   `owed` (`ManaPayment`), `casting`, `targeting`, `sorcery_lock`,
//!   `sorceries_have_flash`, `SeatView::{speed, rad, no_max_hand_size}`, and
//!   `ManaPoolView::{spending, restricted}`.
//! - **`CastModeKind` & option `ManaCost` features** on `ChooseMode`
//!   candidates.
//! - **Expanded neural head coverage** across all 28 `Pending` variants,
//!   including `ChooseManaAbility`, `ChooseDamageSource`, `ChooseDamageEffect`,
//!   `TakeGrantedAction`, `ChooseCardName`, `CancelCast`, `ChoosePile`, and
//!   `Arrange`, with an explicit [`CoverageMatrix4`] and per-step provenance.
//! - **View-only state hash** ([`view_only_state_hash`]) for stall/loop guards
//!   that never reads hidden engine state (`Session::snapshot_hash`).

// Counts and coordinates are deliberately normalized into the f32 model ABI.
#![allow(clippy::cast_precision_loss)]

use std::collections::BTreeMap;

use baylee_core::color::Color;
use baylee_core::ids::{CardIndex, Defender, ObjectId, PlayerId};
use baylee_core::mana::{ManaCost, ManaPayment, ManaSpending};
use baylee_core::preset::GamePreset;
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_engine::choice::{
    ArrangePile, CastModeKind, DamageEffectKind, LegalActions, Pending, PlayerAction,
    default_arrangement,
};
use baylee_view::{
    CounterKind, DayNight, HandObject, ObjectStatus, Phase, PlayerView, PublicObject, StackItem,
    Step, TargetRef,
};
use serde::{Deserialize, Serialize};

use crate::carddsl::{
    CARD_DSL_WIDTH, UNKNOWN_CARD, encode_ability_def, encode_card_face, encode_mana_cost,
    encode_token_def,
};

/// Packs an [`ObjectId`] (`slot` + `generation`) into a deterministic `u64`.
#[inline]
#[must_use]
pub const fn pack_object_id(id: ObjectId) -> u64 {
    ((id.generation() as u64) << 32) | (id.slot() as u64)
}

/// Schema version of the v4 state/action encoder.
pub const ENCODER_VERSION: u32 = 6;

/// Maximum seats at a Baylee table.
pub const MAX_SEATS: usize = 8;

/// Maximum tokenized entities per frame.
pub const MAX_ENTITIES: usize = 1024;

/// Maximum directed relational edges per frame.
pub const MAX_EDGES: usize = 8192;

/// Maximum action candidates per decision.
pub const MAX_CANDIDATES: usize = 65535;

/// Maximum pair candidates for `PairPick` (`DeclareBlockers`, multi-defender attacks).
pub const MAX_PAIRS: usize = 65535;

/// Width of per-entity numeric features (excluding looked-up `CARD_DSL_WIDTH`).
pub const ENTITY_FEAT_WIDTH: usize = 96;

/// Width of per-seat numeric features (`8 x 64`).
pub const SEAT_FEAT_WIDTH: usize = 64;

/// Width of the global table/context token (`64`).
pub const GLOBAL_FEAT_WIDTH: usize = 64;

/// Width of each action candidate's numeric feature vector (`48`).
pub const CAND_FEAT_WIDTH: usize = 48;

/// Number of directed edge relation types (`0..14`).
pub const NUM_EDGE_RELATIONS: usize = 14;

/// Token offset of seat 0 in the unified transformer sequence (`0` is Global/CLS).
pub const SEAT_TOKEN_OFFSET: i16 = 1;

/// Token offset of entity 0 in the unified transformer sequence (`1..=8` are seats).
pub const ENTITY_TOKEN_OFFSET: i16 = 9;

#[inline]
fn unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(0.0, 1.0)
}

#[inline]
fn signed_unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(-1.0, 1.0)
}

/// Relative seat index `0..7` clockwise from `viewer` (`0` = `viewer`), or `8`
/// when `seat` is `None`.
#[inline]
#[must_use]
pub fn rel_seat(viewer: PlayerId, seat: Option<PlayerId>, seat_count: usize) -> usize {
    let Some(s) = seat else {
        return MAX_SEATS;
    };
    let n = seat_count.max(1);
    ((s.get() as usize) + n - (viewer.get() as usize % n)) % n
}

/// Inverse of [`rel_seat`]: maps relative seat `0..seat_count` back to [`PlayerId`].
#[inline]
#[must_use]
pub fn abs_seat(viewer: PlayerId, rel: usize, seat_count: usize) -> PlayerId {
    let n = seat_count.max(1);
    PlayerId::new(((viewer.get() as usize + rel) % n) as u8)
}

/// Consistent multiplayer side utility:
/// - `1.0` if `seat`'s side won,
/// - `0.0` if another side won,
/// - `1.0 / surviving_sides` on a draw if `seat`'s side survived, else `0.0`.
#[must_use]
pub fn side_utility(seat: u8, teams: &[Option<u8>], surviving_seats: &[u8], winners: &[u8]) -> f32 {
    let my_team = teams.get(seat as usize).copied().flatten();
    let on_my_side = |other: u8| -> bool {
        if other == seat {
            return true;
        }
        match (my_team, teams.get(other as usize).copied().flatten()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    };
    if !winners.is_empty() {
        return if winners.iter().copied().any(on_my_side) {
            1.0
        } else {
            0.0
        };
    }
    // Draw: split among surviving sides
    let my_side_survived = surviving_seats.iter().copied().any(on_my_side);
    if !my_side_survived {
        return 0.0;
    }
    let mut sides: Vec<u16> = Vec::new();
    for &s in surviving_seats {
        let side_key = match teams.get(s as usize).copied().flatten() {
            Some(t) => 0x100 | u16::from(t),
            None => u16::from(s),
        };
        if !sides.contains(&side_key) {
            sides.push(side_key);
        }
    }
    1.0 / (sides.len().max(1) as f32)
}

// Kept in one namespace; these files separate observation, state and action contracts.
include!("knowledge.rs");
include!("state.rs");
include!("actions.rs");
include!("candidates.rs");
include!("attack_subset.rs");
include!("targets.rs");
include!("sampling.rs");
include!("tests.rs");
