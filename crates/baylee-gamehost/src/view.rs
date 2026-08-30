//! Building per-seat views (CR 400.2) from engine state.
//!
//! The wire types live in [`baylee_view`] so that clients do not have to link
//! the rules kernel. This module is the only place that translates engine
//! state into them, and it is where the hidden-information rules are enforced:
//! a seat sees public zones in full, its own hand, and only counts for hidden
//! zones belonging to anyone else.
//!
//! Characteristics are taken from the engine's **projected** values, not from
//! the printed card. A client cannot run the layer system, so an anthem, a
//! clone, or an animated land has to arrive already resolved.

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::object::{GameObject, ObjectKind};
use baylee_engine::state::GameState;
use baylee_engine::turn::{Phase as EnginePhase, Step as EngineStep};
use baylee_engine::zone::ZoneLocation;
use baylee_view::{
    AttackerView, BlockerView, CardIdentity, CombatView, CounterEntry, CounterKind, GameStatic,
    HandObject, ObjectStatus, Phase, PlayerView, PublicObject, SeatView, Step, TargetRef,
};

pub use baylee_view as wire;

/// Translates the engine's phase into the wire enum.
const fn phase(p: EnginePhase) -> Phase {
    match p {
        EnginePhase::Beginning => Phase::Beginning,
        EnginePhase::FirstMain => Phase::FirstMain,
        EnginePhase::Combat => Phase::Combat,
        EnginePhase::SecondMain => Phase::SecondMain,
        EnginePhase::Ending => Phase::Ending,
    }
}

/// Translates the engine's step into the wire enum.
const fn step(s: EngineStep) -> Step {
    match s {
        EngineStep::Untap => Step::Untap,
        EngineStep::Upkeep => Step::Upkeep,
        EngineStep::Draw => Step::Draw,
        EngineStep::Main => Step::Main,
        EngineStep::CombatBegin => Step::CombatBegin,
        EngineStep::DeclareAttackers => Step::DeclareAttackers,
        EngineStep::DeclareBlockers => Step::DeclareBlockers,
        EngineStep::CombatDamageFirst => Step::CombatDamageFirst,
        EngineStep::CombatDamage => Step::CombatDamage,
        EngineStep::CombatEnd => Step::CombatEnd,
        EngineStep::End => Step::End,
        EngineStep::Cleanup => Step::Cleanup,
    }
}

/// Translates a counter kind into the wire enum.
const fn counter(kind: baylee_cards_dsl::CounterKind) -> CounterKind {
    use baylee_cards_dsl::CounterKind as K;
    match kind {
        K::P1P1 => CounterKind::PlusOnePlusOne,
        K::M1M1 => CounterKind::MinusOneMinusOne,
        K::Loyalty => CounterKind::Loyalty,
        K::Lore => CounterKind::Lore,
        K::Time => CounterKind::Time,
        K::Charge => CounterKind::Charge,
        K::Poison => CounterKind::Poison,
        K::Energy => CounterKind::Energy,
        K::Rad => CounterKind::Rad,
        K::Lifelink => CounterKind::Lifelink,
        K::Level => CounterKind::Level,
        K::Custom(id) => CounterKind::Custom(id as u32),
    }
}

/// Whether `seat` is entitled to know what card backs `obj`.
///
/// Face-down permanents are the one case where two seats looking at the same
/// battlefield legitimately see different things (CR 707.2): the controller
/// knows what they played, everyone else sees a blank. Returning `None` for
/// the card identity — rather than sending it and trusting the client to hide
/// it — is what makes the leak unrepresentable.
fn may_know_card(obj: &GameObject, seat: PlayerId) -> bool {
    !obj.status
        .contains(baylee_engine::object::Status::FACE_DOWN)
        || obj.controller == seat
}

/// The public name of an object as `seat` may know it.
fn public_name(state: &GameState, obj: &GameObject, seat: PlayerId) -> String {
    if may_know_card(obj, seat) {
        state.names.get(obj.characteristics().name).to_string()
    } else {
        "Face-down".to_string()
    }
}

/// Projects one object into its public form for `seat`.
fn public_object(state: &GameState, id: ObjectId, seat: PlayerId) -> Option<PublicObject> {
    let obj = state.object(id)?;
    let chars = obj.characteristics();
    let known = may_know_card(obj, seat);
    Some(PublicObject {
        id,
        card: obj.card.filter(|_| known).map(|c| CardIdentity {
            index: c.index,
            print: c.print,
            face: obj.face_index,
        }),
        name: public_name(state, obj, seat),
        controller: obj.controller,
        owner: obj.owner,
        status: ObjectStatus::from_bits(obj.status.bits()),
        types: chars.types,
        supertypes: chars.supertypes,
        colors: chars.colors,
        keywords: chars.keywords.bits(),
        power: chars.power,
        toughness: chars.toughness,
        loyalty: chars.loyalty,
        damage: obj.damage,
        counters: obj
            .counters
            .iter()
            .map(|(kind, count)| CounterEntry {
                kind: counter(kind),
                count,
            })
            .collect(),
        attached_to: obj.attached_to,
        targets: obj.targets.iter().map(|t| TargetRef::Object(*t)).collect(),
        summoning_sick: obj.kind == ObjectKind::Permanent
            && baylee_engine::combat::summoning_sick(state, obj),
    })
}

/// Collects a public zone into view objects.
fn zone(state: &GameState, loc: ZoneLocation, seat: PlayerId) -> Vec<PublicObject> {
    state
        .zones
        .list(loc)
        .iter()
        .filter_map(|id| public_object(state, *id, seat))
        .collect()
}

/// Collects one zone per seat, indexed by seat order.
fn per_seat_zone(
    state: &GameState,
    loc: fn(PlayerId) -> ZoneLocation,
    seat: PlayerId,
) -> Vec<Vec<PublicObject>> {
    state
        .players
        .iter()
        .map(|p| zone(state, loc(p.id), seat))
        .collect()
}

/// Builds the hidden-information-filtered view of `state` for `seat`.
///
/// `priority` is the seat that currently holds priority, which the engine
/// tracks in its pending choice rather than in the state itself.
#[must_use]
pub fn player_view(
    state: &GameState,
    seat: PlayerId,
    priority: Option<PlayerId>,
    seq: u64,
) -> PlayerView {
    let hand = state
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .filter_map(|id| {
            let obj = state.object(*id)?;
            let card = obj.card?;
            let chars = obj.characteristics();
            Some(HandObject {
                id: *id,
                card: CardIdentity {
                    index: card.index,
                    print: card.print,
                    face: obj.face_index,
                },
                name: state.names.get(chars.name).to_string(),
                mana_value: chars.mana_cost.cmc(),
                colors: chars.colors,
                types: chars.types,
            })
        })
        .collect();

    PlayerView {
        seq,
        seat,
        turn: state.turn.number,
        phase: phase(state.turn.phase),
        step: step(state.turn.step),
        active: state.turn.active,
        priority,
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
                graveyard_count: state.zones.list(ZoneLocation::Graveyard(p.id)).len() as u32,
                has_lost: p.has_lost,
                commander_casts: state.commander_casts.clone(),
            })
            .collect(),
        hand,
        battlefield: zone(state, ZoneLocation::Battlefield, seat),
        stack: zone(state, ZoneLocation::Stack, seat),
        graveyards: per_seat_zone(state, ZoneLocation::Graveyard, seat),
        exile: per_seat_zone(state, ZoneLocation::Exile, seat),
        command: per_seat_zone(state, ZoneLocation::Command, seat),
        combat: CombatView {
            attackers: state
                .combat
                .attackers
                .iter()
                .map(|a| AttackerView {
                    creature: a.creature,
                    defending: a.defending,
                })
                .collect(),
            blockers: state
                .combat
                .blockers
                .iter()
                .map(|b| BlockerView {
                    blocker: b.blocker,
                    attacker: b.attacker,
                })
                .collect(),
        },
    }
}

/// Builds the once-per-game static payload a client needs before it can render
/// anything: who sits where, and the print table its images are keyed by.
#[must_use]
pub fn game_static(
    game_id: String,
    your_seat: PlayerId,
    seats: Vec<baylee_view::SeatIdentity>,
    prints: &[baylee_core::preset::PrintInfo],
) -> GameStatic {
    GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id,
        your_seat,
        seats,
        prints: prints
            .iter()
            .map(|p| baylee_view::PrintEntry {
                scryfall_id: p.scryfall_id.to_string(),
                lang: p.lang.clone(),
                finish: match p.finish {
                    baylee_core::preset::Finish::Foil => baylee_view::Finish::Foil,
                    baylee_core::preset::Finish::Etched => baylee_view::Finish::Etched,
                    baylee_core::preset::Finish::Normal => baylee_view::Finish::Normal,
                },
            })
            .collect(),
    }
}
