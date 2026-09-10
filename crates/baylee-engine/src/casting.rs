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
use baylee_core::mana::{ManaColor, ManaCost, ManaPool};
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

/// Whether any effect lets mana be spent as though it were mana of any
/// color (Mycosynth Lattice).
///
/// The payment step has always honoured this; the *affordability* checks
/// did not, so the Lattice's third line was unreachable — the spell it made
/// payable was never offered as castable in the first place.
#[must_use]
pub fn mana_is_wild(state: &GameState) -> bool {
    state
        .effects
        .iter()
        .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::ManaIsAnyColor))
}

/// The permanents convoke may be paid with (CR 702.51): untapped creatures
/// and artifacts the caster controls, one `{1}` each.
///
/// One function because the offer and the payment must not disagree. The
/// wizard enumerated these to ask which to tap, and `can_cast` did not
/// count them at all — so a convoke spell was offered as castable exactly
/// when its printed cost was already payable, which is the one case convoke
/// is not for. Clever Concealment and Spirit Water Revival were
/// `Coverage::Implemented` and could never actually be convoked.
#[must_use]
pub fn convoke_sources(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.controller == player
                    && (o.characteristics().types.contains(TypeSet::CREATURE)
                        || o.characteristics().types.contains(TypeSet::ARTIFACT))
                    && !o.status.contains(crate::object::Status::TAPPED)
            })
        })
        .collect()
}

/// Whether `pool` covers `cost`, honouring a mana-conversion effect.
pub(crate) fn affordable(state: &GameState, pool: &ManaPool, cost: &ManaCost) -> bool {
    wild_or_not(mana_is_wild(state), pool, cost)
}

/// The pool a *particular spell* may be paid from, or `None` when that is
/// simply the player's pool.
///
/// Restricted mana — Cavern of Souls, Path of Ancestry — does not live in the
/// pool's plain counters, and [`mana_pay::can_pay`] reads nothing else. So a
/// player whose only mana came off a Cavern was offered **nothing to cast**,
/// while `Engine::spend_restricted` on the far side of the wizard would have
/// paid the cost with it without complaint. That is the whole of the fault:
/// the offer and the payment were answering different questions about the
/// same pool.
///
/// They ask the same one here. An entry counts exactly when its own filter
/// matches this spell — the test `spend_restricted` applies, against the same
/// `restriction_info` — so a Cavern naming Ally pays for an Ally and stays
/// invisible to everything else at the table.
///
/// The card is still in a hand rather than on the stack, which is the one
/// difference from the payment site and does not reach these filters: they
/// read characteristics and a chosen subtype, neither of which the stack
/// confers. A rider (uncounterable) is not consulted at all — a rider changes
/// what the spell *becomes*, never whether it can be cast.
/// It is `pub(crate)` because it has **two** callers and they must not
/// disagree: the wizard enumerates the ways to cast a spell with the same
/// probe `can_cast` used to offer it, or a spell is offered in
/// `LegalActions` and then refused as "no way to cast this spell" — which is
/// exactly what the convoke count above this one was written to stop
/// happening.
pub(crate) fn spendable_pool(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> Option<ManaPool> {
    let pool = &state.players[player.get() as usize].mana_pool;
    if pool.restricted().is_empty() {
        return None;
    }
    let spell = state.object(card)?;
    let mut probe = pool.clone();
    for mana in pool.restricted() {
        let Some(&(source, filter, _)) = state.restriction_info.get(&mana.restriction.0) else {
            continue;
        };
        if crate::eval::matches(filter, state, spell, player, source) {
            probe.add(mana.color, mana.amount);
        }
    }
    Some(probe)
}

/// [`affordable`] with the conversion flag already read.
///
/// Payment sites need it in this shape: `pool` is borrowed mutably there,
/// so the state cannot be read at the same time.
pub(crate) fn wild_or_not(wild: bool, pool: &ManaPool, cost: &ManaCost) -> bool {
    if wild {
        mana_pay::can_pay_wild(pool, cost)
    } else {
        mana_pay::can_pay(pool, cost)
    }
}

/// Pays `cost` from `pool`, honouring a mana-conversion effect.
///
/// `wild` comes from [`mana_is_wild`], read before the pool is borrowed.
pub(crate) fn pay_with(wild: bool, pool: &mut ManaPool, cost: &ManaCost) -> bool {
    if wild {
        mana_pay::pay_wild(pool, cost)
    } else {
        mana_pay::pay(pool, cost)
    }
}

/// Whether `player` may begin casting a spell with these characteristics
/// right now.
///
/// CR 117.1a is the permission: an instant any time its controller has
/// priority, a noninstant during their own main phase with the stack empty
/// (CR 307.1 says the same of a sorcery in particular). Flash (CR 702.8a)
/// moves a card into the first group whatever its types say, Teferi's +1
/// does the same for that player's sorceries, and Teferi's static pulls
/// every opponent's spell back into the second.
///
/// Characteristics rather than an object, because two callers ask this
/// about two different things. [`can_cast`] passes the *projected*
/// characteristics of a card that is sitting in a zone, so a continuous
/// effect that granted it flash is read. The prepared cast passes the
/// **printed** face of a card that has no object at all — the copy does not
/// exist until it is cast, so nothing could have granted it anything — and
/// still wants the two player-scoped effects above, which is the whole
/// reason this is one function and not two.
pub(crate) fn timing_allows(
    state: &GameState,
    player: PlayerId,
    types: TypeSet,
    keywords: baylee_cards_dsl::KeywordSet,
) -> bool {
    // Teferi's restriction forces sorcery-speed timing on everything for
    // opponents.
    let teferi_lock = state.effects.iter().any(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::OpponentsCastAsSorcery
        ) && state.is_opponent(fx.controller, player)
    });
    // Flash (CR 702.8a) makes a card castable whenever an instant could be
    // — Snapcaster Mage, Restoration Angel, the whole free-spell cycle.
    let is_instant =
        types.contains(TypeSet::INSTANT) || keywords.contains(baylee_cards_dsl::KeywordSet::FLASH);
    // Teferi +1: your sorceries have flash until your next turn.
    let sorcery_flash = types.contains(TypeSet::SORCERY)
        && state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::SorceriesHaveFlash)
                && fx.controller == player
        });
    if teferi_lock || (!is_instant && !sorcery_flash) {
        let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        return main_phase && state.turn.active == player && state.zones.stack_is_empty();
    }
    true
}

/// Whether `card` can be cast by `player` right now (printed cost or any
/// alternative/mode).
///
/// # Errors
/// [`CastError`] describing the first legality violation.
#[allow(clippy::too_many_lines)] // one gate per rule; splitting hides the list
pub fn can_cast(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastError> {
    let obj = state.object(card).ok_or(CastError::NotInHand)?;
    let in_hand = obj.zone == Zone::Hand && obj.zone_owner == Some(player);
    let in_own_graveyard = obj.zone == Zone::Graveyard && obj.zone_owner == Some(player);
    // Flashback (CR 702.34): a granted card may be cast from its owner's
    // graveyard.
    let flashback_ok =
        !in_hand && in_own_graveyard && state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::GrantsFlashback)
                && matches!(&fx.filter, crate::effects::EffectFilter::ObjectIs(id) if *id == card)
        });
    // Disturb (CR 702.112): a face with disturb is castable from the
    // owner's graveyard.
    let disturb_ok = !in_hand
        && in_own_graveyard
        && obj
            .card
            .and_then(|c| lookup.card(c.index))
            .is_some_and(|def| def.faces.iter().any(|f| f.disturb));
    // Adventure (CR 715): a card on an adventure may be cast from exile.
    let adventure_ok = !in_hand
        && obj.zone == Zone::Exile
        && obj.riders.contains(&crate::object::Rider::Adventure);
    // Opposition Agent: cards exiled by the takeover are playable by the
    // agent from exile.
    let takeover_ok = !in_hand
        && obj.zone == Zone::Exile
        && obj
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::PlayableFromExileFor(p) if *p == player));
    // Commander (CR 903.8): a commander in the command zone may be cast
    // from there by its owner, at the same timing it would have from a
    // hand. The marker list is what makes it a commander — an emblem is in
    // the same zone and is not castable by anybody.
    let commander_ok = !in_hand
        && obj.zone == Zone::Command
        && obj.zone_owner == Some(player)
        && state
            .commanders
            .get(player.get() as usize)
            .is_some_and(|cs| cs.iter().any(|c| c.object == card));
    if !in_hand && !flashback_ok && !disturb_ok && !adventure_ok && !takeover_ok && !commander_ok {
        return Err(CastError::NotInHand);
    }
    let c = obj.characteristics();
    // Lands can never be cast as spells (CR 305.1).
    if c.types.contains(TypeSet::LAND) {
        return Err(CastError::BadTiming);
    }
    // Timing (CR 601.3). Read off the projected characteristics, so a
    // granted flash counts.
    if !timing_allows(state, player, c.types, c.keywords) {
        return Err(CastError::BadTiming);
    }
    // Restricted mana this spell may be paid with counts towards it; see
    // [`spendable_pool`].
    let with_restricted = spendable_pool(state, player, card);
    let pool = with_restricted
        .as_ref()
        .unwrap_or(&state.players[player.get() as usize].mana_pool);
    // Commander tax (CR 903.8). A cost *increase*, so it lands on every way
    // of casting the card — printed cost, alternative cost and mode alike
    // (CR 601.2f) — which is why it is folded into each probe below rather
    // than into the first one.
    let tax = commander_tax(state, player, card);
    // Convoke (CR 702.51) is a *reduction* of the generic part, so it goes
    // on the same probes the tax does and in the other direction. Read off
    // the printed face: a granted convoke does not exist.
    let convoke = obj
        .card
        .and_then(|c| lookup.card(c.index))
        .filter(|def| def.faces[0].convoke)
        .map_or(0, |_| convoke_sources(state, player).len() as u32);
    let probe = |cost: &ManaCost| {
        affordable(
            state,
            pool,
            &cost.with_more_generic(tax).with_less_generic(convoke),
        )
    };
    // Printed cost probed with X = 0; the full payment is validated when
    // the wizard finishes.
    if !probe(&c.mana_cost.with_x(0)) {
        // Alternative costs may still make it castable (pitch/evoke). The
        // wizard computes the exact options; this decides only whether there
        // is one, and asks about the whole cost to do it.
        let Some(card_ref) = obj.card else {
            return Err(CastError::NotEnoughMana);
        };
        let Some(def) = lookup.card(card_ref.index) else {
            return Err(CastError::NotEnoughMana);
        };
        let face = &def.faces[0];
        // Mana is not the whole of an alternative cost, and this probe used
        // to behave as though it were: a Force of Will with no other blue
        // card in hand has a zero mana cost, so it went into
        // `legal.castable` and the wizard then reversed the cast at the pitch
        // stage. Every part is asked about here, the way every condition is
        // asked about below — the two halves of the same offer.
        let any_alt = face.alternative_costs.iter().any(|alt| {
            // Every condition, not merely the one that was written first.
            // `CommanderControlled` fell through the old `!matches!` as
            // "true", so Flawless Maneuver and Fierce Guardianship were
            // offered as castable with no commander anywhere — and the
            // wizard, which does check the condition, then found no way to
            // cast them at all.
            let condition_ok = match alt.condition {
                baylee_cards_dsl::AltCondition::Always => true,
                baylee_cards_dsl::AltCondition::NotYourTurn => state.turn.active != player,
                baylee_cards_dsl::AltCondition::CommanderControlled => {
                    controls_a_commander(state, player)
                }
            };
            condition_ok
                && probe(&alt.cost.mana)
                && alternative_parts_payable(state, player, card, alt.cost.parts)
        });
        let any_mode = def.abilities.iter().any(|a| match a {
            baylee_cards_dsl::AbilityDef::ModalSpell { modes } => modes
                .iter()
                .any(|m| probe(&m.cost_override.unwrap_or(face.mana_cost).with_x(0))),
            _ => false,
        });
        if !any_alt && !any_mode {
            return Err(CastError::NotEnoughMana);
        }
    }
    Ok(())
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
        obj.set_controller(player);
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

/// Mana color produced by a land with exactly one basic land subtype
/// (CR 305.6).
///
/// `None` for a land with no basic subtype and for one with several: both
/// are served by the `AddManaChoice` ability printed on the card, which can
/// offer the choice this shortcut cannot.
#[must_use]
pub fn intrinsic_mana(state: &GameState, source: ObjectId) -> Option<ManaColor> {
    let obj = state.object(source)?;
    if !obj.characteristics().types.contains(TypeSet::LAND) {
        return None;
    }
    let s = &obj.characteristics().subtypes;
    let mut only = None;
    for (subtype, color) in [
        (land::PLAINS, ManaColor::White),
        (land::ISLAND, ManaColor::Blue),
        (land::SWAMP, ManaColor::Black),
        (land::MOUNTAIN, ManaColor::Red),
        (land::FOREST, ManaColor::Green),
    ] {
        if s.contains(subtype) {
            // CR 305.6 gives a land one mana ability *per* basic type, so a
            // dual has two of them and its controller picks which to
            // activate. This shortcut cannot ask, and answering on the
            // player's behalf is worse than not answering at all: Godless
            // Shrine used to tap for white and never for black, whatever the
            // player needed. A land with more than one basic type is left to
            // the printed `AddManaChoice` ability on its card, which does ask.
            if only.is_some() {
                return None;
            }
            only = Some(color);
        }
    }
    only
}

/// CR 903.8's tax on casting `card` from the command zone, in generic mana:
/// `{2}` for each previous cast of *this* commander from there.
///
/// Zero unless the card is in the command zone right now — a commander cast
/// from a hand it was bounced to pays nothing. And per commander rather than
/// per seat: a partner deck taxes its two independently, which is why the
/// count sits on [`crate::state::Commander`] and not beside
/// `GameState::commander_casts`, whose job is Commander's Insight.
///
/// It lives beside [`controls_a_commander`] for the same reason that one
/// does, and it moved here from the wizard after making the same mistake in
/// the other direction: the wizard charged the tax and the legality probe
/// did not, so a commander that had been cast once was *offered* for its
/// printed cost and then refused mid-wizard. A tax the counter records and
/// nothing charges is not a tax, and one only half the engine charges is
/// worse than none.
#[must_use]
pub fn commander_tax(state: &GameState, player: PlayerId, card: ObjectId) -> u32 {
    if state.object(card).map(|o| o.zone) != Some(Zone::Command) {
        return 0;
    }
    state
        .commanders
        .get(player.get() as usize)
        .and_then(|cs| cs.iter().find(|c| c.object == card))
        .map_or(0, |c| c.casts.saturating_mul(2))
}

/// Does `player` control one of their own commanders right now?
///
/// This is card text, not a rule — "if you control a commander" is what
/// Fierce Guardianship and Flawless Maneuver print — but it lives here
/// because two readers ask it: the legality probe below and the wizard's
/// list of cast options. A probe that answered differently from the wizard
/// offers a spell the wizard then refuses to cast, which is exactly the bug
/// this replaces. It reads the marker list rather than the command zone,
/// since a commander on the battlefield has left that zone by definition.
#[must_use]
pub fn controls_a_commander(state: &GameState, player: PlayerId) -> bool {
    state
        .commanders
        .get(player.get() as usize)
        .is_some_and(|cs| {
            cs.iter().any(|c| {
                state
                    .object(c.object)
                    .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == player)
            })
        })
}

/// The cards that could be exiled from `player`'s hand to pay an
/// `ExileFromHand` cost on `card` — Force of Will's blue card, Solitude's
/// white one.
///
/// One reader, three callers, and that is the whole point of it being a
/// function. `cast_wizard`'s `PitchChoice` stage builds its prompt from this
/// list; [`can_cast`] asks whether the list is empty before calling the card
/// castable; `Engine::can_afford` asks the same before offering the
/// alternative cost as a mode. The two askers used to answer "yes" without
/// looking, so a Force of Will with no other blue card was lit up as castable
/// and then reversed itself on reaching the pitch stage — the shape a dead
/// offer always has, and the one a second predicate written to *agree* with
/// the stage would have kept, because it would have agreed with itself.
///
/// The card being cast is not a candidate: it is what is being paid for, and
/// it is what `eval::matches` is handed as `this`, so a filter saying
/// "another" reads the same word here as anywhere else.
#[must_use]
pub fn pitchable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    filter: &baylee_cards_dsl::Filter,
) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .copied()
        .filter(|id| {
            *id != card
                && state
                    .object(*id)
                    .is_some_and(|o| crate::eval::matches(filter, state, o, player, card))
        })
        .collect()
}

/// Whether the non-mana parts of an alternative cost could be paid right now.
///
/// [`can_cast`] reaches this only when the printed cost is unaffordable and
/// the face prints an alternative cost, which in this pool is four cards, so
/// the hand scan costs nothing anybody can measure. `zones.list` is ordered,
/// so it costs no determinism either.
///
/// The nine parts that answer `true` are named rather than swept into a
/// wildcard: `cast_wizard::paid_as_an_alternative_cost` says which two of
/// them the payment ever pays, and `offer_tests` holds the rest off this list
/// entirely — so a twelfth `CostPart` has to be looked at here too rather
/// than being quietly declared payable.
fn alternative_parts_payable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    parts: &[baylee_cards_dsl::CostPart],
) -> bool {
    use baylee_cards_dsl::CostPart;
    parts.iter().all(|part| match part {
        CostPart::PayLife(n) => state.can_pay_life(player, i32::from(*n)),
        CostPart::ExileFromHand(filter) => !pitchable(state, player, card, filter).is_empty(),
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::PayLifeX => true,
    })
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
        // A land is never summoning sick, so this costs an ordinary land
        // nothing; it is here for the land that is also a creature (Dryad
        // Arbor, an animated manland), whose intrinsic {T} is an activated
        // ability of a creature like any other (CR 302.6).
        && !crate::combat::summoning_sick(state, obj)
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
