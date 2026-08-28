//! Casting and resolution.
//!
//! S2 scope: no modes/targets/X, auto-payment (CR 601.2h compressed).
//! The full stepwise `CastPlan` wizard (modes, alternative/additional
//! costs, targets, X, payment plans) lands with the ability runtime (M1.S3
//! / M2).

use crate::event::{Cause, GameEvent};
use crate::mana_pay;
use crate::object::ObjectKind;
use crate::state::{GameState, StateError};
use crate::turn::Phase;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_core::types::TypeSet;

/// Why a card cannot be cast right now (validated before the action).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CastError {
    /// The card is not in the caster's hand.
    #[error("card is not in your hand")]
    NotInHand,
    /// Timing rules forbid it (main phase, own turn, empty stack).
    #[error("sorcery-speed timing not met")]
    BadTiming,
    /// The mana pool cannot pay the cost.
    #[error("not enough mana")]
    NotEnoughMana,
    /// Costs with {X}/{Y}/{Z} need the full wizard (M1.S3).
    #[error("variable costs not supported yet")]
    VariableCost,
}

/// Whether `card` can be cast by `player` right now (S2 rules).
///
/// # Errors
/// [`CastError`] describing the first legality violation.
pub fn can_cast(state: &GameState, player: PlayerId, card: ObjectId) -> Result<(), CastError> {
    let obj = state.object(card).ok_or(CastError::NotInHand)?;
    if obj.zone != Zone::Hand || obj.zone_owner != Some(player) {
        return Err(CastError::NotInHand);
    }
    let c = obj.characteristics();
    // Timing (CR 601.3): permanents and sorceries are sorcery-speed;
    // instants (flash later) are any-time.
    let is_instant = c.types.contains(TypeSet::INSTANT);
    if !is_instant {
        let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        if !main_phase || state.turn.active != player || !state.zones.stack_is_empty() {
            return Err(CastError::BadTiming);
        }
    }
    if c.mana_cost.has_variable() {
        return Err(CastError::VariableCost);
    }
    let pool = &state.players[player.get() as usize].mana_pool;
    if !mana_pay::can_pay(pool, &c.mana_cost) {
        return Err(CastError::NotEnoughMana);
    }
    Ok(())
}

/// Casts a spell: pays the cost, moves the card to the stack.
///
/// # Errors
/// [`CastError`] on legality, [`StateError`] on stale handles.
///
/// # Panics
/// Internal invariant violations (existence validated first).
pub fn cast_spell(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
) -> Result<ObjectId, CastFailure> {
    can_cast(state, player, card).map_err(CastFailure::Legality)?;
    let cost = state
        .object(card)
        .expect("validated")
        .characteristics()
        .mana_cost;
    let pool = &mut state.players[player.get() as usize].mana_pool;
    debug_assert!(mana_pay::pay(pool, &cost), "payment validated by can_cast");
    {
        let obj = state.object_mut(card).expect("validated");
        obj.kind = ObjectKind::Spell;
        obj.controller = player;
    }
    state
        .move_object(card, ZoneLocation::Stack, ZonePosition::Top, Cause::Spell)
        .map_err(CastFailure::State)?;
    state.journal.record(GameEvent::SpellCast {
        object: card,
        player,
    });
    Ok(card)
}

/// Resolves the top object of the stack (S2: no effects — permanents enter
/// the battlefield, instants/sorceries go to the graveyard).
///
/// # Panics
/// Internal invariant violations (stack objects always exist).
pub fn resolve_top(state: &mut GameState) {
    let Some(&top) = state.zones.list(ZoneLocation::Stack).last() else {
        return;
    };
    let (is_permanent, controller) = {
        let obj = state.object(top).expect("stack object exists");
        (obj.characteristics().types.is_permanent(), obj.controller)
    };
    state
        .journal
        .record(GameEvent::StackObjectResolved { object: top });
    if is_permanent {
        if let Some(obj) = state.object_mut(top) {
            obj.kind = ObjectKind::Permanent;
            obj.status.remove(crate::object::Status::TAPPED);
        }
        let _ = state.move_object(
            top,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Spell,
        );
        let _ = controller;
    } else {
        let owner = state.object(top).map_or(controller, |o| o.owner);
        if let Some(obj) = state.object_mut(top) {
            obj.kind = ObjectKind::Card;
        }
        let _ = state.move_object(
            top,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Spell,
        );
    }
}

/// Plays a land (special action, no stack).
///
/// # Errors
/// [`CastFailure`] when the action is illegal.
///
/// # Panics
/// Internal invariant violations (existence validated first).
pub fn play_land(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastFailure> {
    let obj = state.object(card).ok_or(CastFailure::NoSuchObject)?;
    if obj.zone != Zone::Hand || obj.zone_owner != Some(player) {
        return Err(CastFailure::Legality(CastError::NotInHand));
    }
    if !obj.characteristics().types.contains(TypeSet::LAND) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
    if !main_phase || state.turn.active != player || !state.zones.stack_is_empty() {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    if state.players[player.get() as usize].lands_played_this_turn >= 1 {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    state.players[player.get() as usize].lands_played_this_turn += 1;
    {
        let obj = state.object_mut(card).expect("validated");
        obj.kind = ObjectKind::Permanent;
        obj.controller = player;
    }
    state
        .move_object(
            card,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .map_err(CastFailure::State)?;
    state.journal.record(GameEvent::LandPlayed {
        object: card,
        player,
    });
    Ok(())
}

/// Mana color produced by a basic land subtype (CR 305.6), `None` for
/// nonbasic lands (their printed abilities arrive with the DSL, M1.S3+).
#[must_use]
pub fn intrinsic_mana(state: &GameState, source: ObjectId) -> Option<ManaColor> {
    let obj = state.object(source)?;
    if !obj.characteristics().types.contains(TypeSet::LAND) {
        return None;
    }
    let s = &obj.characteristics().subtypes;
    if s.contains(land::FOREST) {
        Some(ManaColor::Green)
    } else if s.contains(land::ISLAND) {
        Some(ManaColor::Blue)
    } else if s.contains(land::PLAINS) {
        Some(ManaColor::White)
    } else if s.contains(land::SWAMP) {
        Some(ManaColor::Black)
    } else if s.contains(land::MOUNTAIN) {
        Some(ManaColor::Red)
    } else {
        None
    }
}

/// Whether the intrinsic mana ability of `source` can be activated now.
#[must_use]
pub fn can_activate_mana(state: &GameState, player: PlayerId, source: ObjectId) -> bool {
    let Some(obj) = state.object(source) else {
        return false;
    };
    obj.zone == Zone::Battlefield
        && obj.controller == player
        && !obj.status.contains(crate::object::Status::TAPPED)
        && intrinsic_mana(state, source).is_some()
}

/// Taps a basic land for its intrinsic mana (CR 305.6).
///
/// # Errors
/// [`CastFailure::NoSuchObject`] for stale handles.
///
/// # Panics
/// Internal invariant violations (legality checked first).
pub fn activate_mana(
    state: &mut GameState,
    player: PlayerId,
    source: ObjectId,
) -> Result<(), CastFailure> {
    if !can_activate_mana(state, player, source) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    let color = intrinsic_mana(state, source).expect("checked above");
    {
        let obj = state.object_mut(source).expect("checked above");
        obj.status.insert(crate::object::Status::TAPPED);
    }
    state.journal.record(GameEvent::ObjectTapped {
        object: source,
        cause: Cause::Cost,
    });
    state.players[player.get() as usize].mana_pool.add(color, 1);
    state.journal.record(GameEvent::ManaProduced {
        player,
        color,
        amount: 1,
        source: Some(source),
    });
    Ok(())
}

/// Casting/playing failure.
#[derive(Debug, thiserror::Error)]
pub enum CastFailure {
    /// Legality violation.
    #[error("illegal action: {0}")]
    Legality(#[from] CastError),
    /// Stale handle.
    #[error("object vanished")]
    NoSuchObject,
    /// Zone machinery error.
    #[error("state error: {0}")]
    State(#[from] StateError),
}
