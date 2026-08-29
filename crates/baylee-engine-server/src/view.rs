//! Per-player hidden-information views (CR 400.2): each seat sees public
//! zones in full, its own hand, and only counts for hidden zones of other
//! players. Serialized as the `view_json` payload of `StateDelta`.

use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
use baylee_engine::state::GameState;
use baylee_engine::zone::{Zone, ZoneLocation};
use serde::{Deserialize, Serialize};

/// A publicly visible object (everything except hidden-zone contents).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicObject {
    /// Engine object id.
    pub id: ObjectId,
    /// Card identity (`None` for tokens/emblems).
    pub card: Option<CardIndex>,
    /// Active face (MDFC).
    pub face: u8,
    /// Controller.
    pub controller: PlayerId,
    /// Owner.
    pub owner: PlayerId,
    /// Tapped/flipped status bits (engine internal, public info).
    pub status_bits: u32,
    /// Damage marked on it.
    pub damage: u16,
    /// Counters as (kind tag, count) pairs.
    pub counters: Vec<(u8, u16)>,
    /// The object it's attached to (equipment/auras).
    pub attached_to: Option<ObjectId>,
}

/// A hidden-zone entry visible only to the owning seat.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandObject {
    /// Engine object id.
    pub id: ObjectId,
    /// Card identity.
    pub card: Option<CardIndex>,
    /// Active face.
    pub face: u8,
}

/// A seat's life/pool/counter line.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeatView {
    /// Seat.
    pub player: PlayerId,
    /// Life.
    pub life: i32,
    /// Poison counters.
    pub poison: u16,
    /// Energy counters.
    pub energy: u16,
    /// Cards in hand (contents are in `hand` for the viewing seat).
    pub hand_count: u32,
    /// Cards in library.
    pub library_count: u32,
    /// Has lost the game.
    pub has_lost: bool,
}

/// The complete per-seat view (hidden-information filtered).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    /// The seat this view is for.
    pub seat: PlayerId,
    /// Turn number.
    pub turn: u32,
    /// Phase tag.
    pub phase: String,
    /// Step tag.
    pub step: String,
    /// Active player.
    pub active: PlayerId,
    /// The monarch, if any.
    pub monarch: Option<PlayerId>,
    /// Per-seat public lines.
    pub seats: Vec<SeatView>,
    /// Your hand (only your seat's contents).
    pub hand: Vec<HandObject>,
    /// Battlefield (public).
    pub battlefield: Vec<PublicObject>,
    /// The stack (public).
    pub stack: Vec<PublicObject>,
    /// Graveyards (public).
    pub graveyards: Vec<Vec<PublicObject>>,
    /// Exile (public).
    pub exile: Vec<PublicObject>,
    /// Command zone (public).
    pub command: Vec<PublicObject>,
}

/// Builds the hidden-information-filtered view for `seat`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn player_view(state: &GameState, seat: PlayerId) -> PlayerView {
    let public = |id: ObjectId| -> Option<PublicObject> {
        let obj = state.object(id)?;
        Some(PublicObject {
            id,
            card: obj.card.map(|c| c.index),
            face: obj.face_index,
            controller: obj.controller,
            owner: obj.owner,
            status_bits: 0,
            damage: obj.damage,
            counters: obj
                .counters
                .iter()
                .map(|(k, n)| (counter_tag(k), n))
                .collect(),
            attached_to: obj.attached_to,
        })
    };
    let per_seat_zone = |loc: fn(PlayerId) -> ZoneLocation| -> Vec<Vec<PublicObject>> {
        state
            .players
            .iter()
            .map(|p| {
                state
                    .zones
                    .list(loc(p.id))
                    .iter()
                    .filter_map(|id| public(*id))
                    .collect()
            })
            .collect()
    };
    let hand = state
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .filter_map(|id| {
            let obj = state.object(*id)?;
            Some(HandObject {
                id: *id,
                card: obj.card.map(|c| c.index),
                face: obj.face_index,
            })
        })
        .collect();
    PlayerView {
        seat,
        turn: state.turn.number,
        phase: format!("{:?}", state.turn.phase),
        step: format!("{:?}", state.turn.step),
        active: state.turn.active,
        monarch: state.monarch,
        seats: state
            .players
            .iter()
            .map(|p| SeatView {
                player: p.id,
                life: p.life,
                poison: p.poison,
                energy: p.energy,
                hand_count: state.zones.list(ZoneLocation::Hand(p.id)).len() as u32,
                library_count: state.zones.list(ZoneLocation::Library(p.id)).len() as u32,
                has_lost: p.has_lost,
            })
            .collect(),
        hand,
        battlefield: state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter_map(|id| public(*id))
            .collect(),
        stack: state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter_map(|id| public(*id))
            .collect(),
        graveyards: per_seat_zone(ZoneLocation::Graveyard),
        exile: state
            .players
            .iter()
            .flat_map(|p| {
                state
                    .zones
                    .list(ZoneLocation::Exile(p.id))
                    .iter()
                    .filter_map(|id| public(*id))
                    .collect::<Vec<_>>()
            })
            .collect(),
        command: state
            .players
            .iter()
            .flat_map(|p| {
                state
                    .zones
                    .list(ZoneLocation::Command(p.id))
                    .iter()
                    .filter_map(|id| public(*id))
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
}

/// Counter-kind tag shared with the state hash (stable on the wire).
fn counter_tag(kind: baylee_cards_dsl::CounterKind) -> u8 {
    use baylee_cards_dsl::CounterKind as K;
    match kind {
        K::P1P1 => 1,
        K::M1M1 => 2,
        K::Loyalty => 3,
        K::Lore => 4,
        K::Time => 5,
        K::Charge => 6,
        K::Poison => 7,
        K::Energy => 8,
        K::Rad => 9,
        K::Lifelink => 10,
        K::Level => 11,
        K::Custom(id) => 100u8.saturating_add((id % 100) as u8),
    }
}

/// Zone of an object, used by the client to place `PublicObject`s.
#[must_use]
#[allow(dead_code)] // client-side placement helper, kept for the gateway
pub fn zone_of(state: &GameState, id: ObjectId) -> Option<Zone> {
    state.object(id).map(|o| o.zone)
}
