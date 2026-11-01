//! Effect resolution: the op interpreter.
//!
//! Spells and abilities resolve by running their [`Effect`] list through a
//! small continuation machine: operations that need a player choice
//! (searches, scry) suspend into a `Pending::ChooseCards` and resume on the
//! answer. Everything runs through the normal event pipeline, so the
//! journal stays complete.

use crate::choice::{ArrangePile, ArrangePlace, ArrangePrompt, ChoicePrompt, Pending, YesNoPrompt};
use crate::engine::cost_wizard;
use crate::eval;
use crate::event::{Cause, DamageTarget, GameEvent};
use crate::object::{Characteristics, GameObject, ObjectKind};
use crate::sba;
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Amount, CostPart, Effect, PlayerRel, SearchDest, TargetSlot, TargetSpec};
use baylee_core::color::ColorSet;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use smallvec::SmallVec;

mod after_entry;
mod attachments;
mod battle_defeat;
mod behold;
mod bird_entry;
mod names;
pub use names::resume_name;
mod chosen;
mod colors;
mod conditional_grants;
mod continuous_subjects;
mod control;
mod control_choice;
mod control_observers;
mod copying;
mod damage_returns;
mod weakest;
use copying::{copy_spell, copy_target_ability};
mod coin;
pub mod coin_stakes;
#[cfg(test)]
mod coin_stakes_tests;
mod combat_watches;
mod counters;
mod delayed_returns;
mod destruction_receipts;
mod draw_debts;
mod equalize;
mod explore;
mod future_attacks;
mod graveyard_choice;
mod greatest_exchange;
mod landwalk;
mod target_groups;
mod untap_suppression;
pub use landwalk::resume_with_subtype;
mod any_player_payment;
mod attacking_tokens;
mod blocked_returns;
mod half_damage;
mod life;
pub(crate) mod linked_counters;
pub mod loot;
mod mana;
pub(crate) mod mana_life_payment;
mod milling;
mod reanimation;
mod reflexive;
mod regeneration;
mod retarget;
mod reveal_put;
mod source_color;
pub(crate) mod subjects;
mod this_way;
mod token_choices;
mod top_card;
mod would_destroy;
pub use subjects::SubjectContext;
mod amass;
mod card_copies;
mod empower;
mod sacrifices;
pub(crate) mod tokens;
mod zones;

mod answers;
mod awaiting;
mod choices;
mod discard;
mod river;
pub use discard::{DiscardReveal, DiscardThen};
use discard::{discard_or_ask, resume_discard_destination};
mod counter_move;
mod directed_search;
#[cfg(test)]
mod directed_search_tests;
mod discard_any;
pub mod dungeon;
mod event_actions;
mod hand_transfer;
mod immediate;
pub(crate) mod linked_reanimation;
mod meld;
pub(crate) mod mutate;
mod permissions;
mod piles;
mod private_hand;
#[cfg(test)]
mod private_hand_tests;
mod private_library;
#[cfg(test)]
mod private_library_tests;
pub mod proliferate;
mod repeated_graveyard_choice;
#[cfg(test)]
mod repeated_graveyard_tests;
#[cfg(test)]
mod reveal_search_tests;
pub mod tap_energy;
pub(crate) mod tap_group;
use permissions::grant_play;
mod graveyard_top;
pub(crate) mod hidden_exile;
mod resume;
mod search;
mod search_group;
mod shared_land;
mod upgrade;

pub use answers::*;
pub use awaiting::*;
use choices::exec_choice;
use immediate::exec_immediate;
pub use piles::*;
pub use resume::*;
use search::{
    Search, begin_search, bottom_in_random_order, find_for, put_found, reveal_until,
    search_library_for_one, within,
};

/// "Untap enchanted creature", "regenerate enchanted creature": what an
/// Aura's own ability does to its host, which it names without targeting.
#[cfg(test)]
mod host_tests;

/// Twiddle's "tap or untap": a tapped target untaps and an untapped one taps.
#[cfg(test)]
mod toggle_tests;

/// Mana Short: "tap all lands target player controls and that player loses
/// all unspent mana". The player is the resolution's chosen one, and
/// nothing of anybody else's is touched.
#[cfg(test)]
mod mana_short_tests;

/// Disintegrate's "it can't be regenerated this turn" (CR 701.19c): the
/// resolution records the target as it is now, and a later destruction —
/// lethal damage, which no card calls unregeneratable — goes through the
/// shield it already had.
#[cfg(test)]
mod no_regeneration_tests;

/// Wheel of Fortune, Timetwister and Natural Selection: whole hands and
/// graveyards moved for each player named, and another player's library
/// looked at and ordered by the ability's controller.
#[cfg(test)]
mod whole_zone_tests;

/// "You may pay {1}. If you do, you gain 1 life" and its mirror, "… unless
/// you pay {1}": one question, one payment, and the effects on opposite
/// answers.
#[cfg(test)]
mod price_tests;

#[cfg(test)]
mod bound_now_tests;

/// Nothing a resolution would create for its controller is created once
/// they have left the game (CR 800.4d), though the resolution goes on
/// without them (CR 608.2m).
#[cfg(test)]
mod created_for_the_departed_tests;

#[cfg(test)]
mod counted_choice_tests;

#[cfg(test)]
mod controller_of_target_tests;
#[cfg(test)]
mod ice_combat_tests;

#[cfg(test)]
mod copied_decisions_tests;
#[cfg(test)]
mod protection_color_tests;

pub use control::resume_control_choice;
pub(crate) use life::{refresh_damage, resume_damage};
/// Which colours a mana source can produce right now, at this board.
///
/// Exported because `baylee-gamehost` projects the answer into the view
/// (`PublicObject::board_mana`) and a client cannot work it out: a Reflecting
/// Pool's colours are the union of `produced_colors` over one side's lands,
/// which is a *projected* characteristic no registry carries, and a Command
/// Tower's are its controller's commanders' identity. A second copy of either
/// fold would be a second answer, and the one that disagreed would be a
/// permanent the planner counts on and the engine refuses.
///
/// This is the same function [`add_mana`](mana) calls on the way to handing
/// the mana out, which is the whole point of exporting it rather than writing
/// a projection beside it.
pub use mana::colors_of;

/// A running effect resolution (continuation).
#[derive(Clone, Debug)]
pub struct Resolution {
    /// The source permanent/spell of the effect.
    pub source: ObjectId,
    /// The stack object being resolved (spell or ability).
    pub on_stack: ObjectId,
    /// Controlling player.
    pub controller: PlayerId,
    /// Flattened effect operations.
    pub effects: Vec<Effect>,
    /// Program counter.
    pub pc: usize,
    /// Targets chosen at cast/activation.
    pub targets: SmallVec<[ObjectId; 2]>,
    /// Targets chosen for a second instance of the word "target", read
    /// through [`baylee_cards_dsl::TargetSlot::Second`] and by nothing that
    /// reads `targets` — see `GameObject::second_targets`.
    pub second_targets: SmallVec<[ObjectId; 1]>,
    /// X value, if any.
    pub x: Option<u32>,
    /// Chosen target player, if any.
    pub chosen_player: Option<PlayerId>,
    /// Players chosen as *targets* ("any target", CR 115.4).
    ///
    /// Distinct from `chosen_player`, which is the single player a
    /// `TargetSpec::AnyPlayer` names: this list rides alongside `targets`
    /// as the other half of one mixed choice, so a spell that hits two
    /// any-targets can hit two players.
    pub target_players: baylee_core::ids::SeatSet,
    /// The object the triggering event was about (event-driven triggers).
    pub event_object: Option<ObjectId>,
    /// Production captured by the triggering mana event.
    pub event_mana: Option<crate::trigger::EventMana>,
    /// The suspended choice, if any.
    pub awaiting: Option<AwaitingOp>,
    /// Whether the thing being resolved said "target" at all.
    ///
    /// [`Filter::This`](baylee_cards_dsl::Filter::This) names two different
    /// objects and this is what tells them apart: the *source* for an ability
    /// that never targeted ("this creature gets +2/+2 until end of turn") and
    /// the *target* for one that did. An empty `targets` used to mean the
    /// first of those unconditionally, which was safe only for as long as a
    /// targeted ability could never resolve without a target. "Up to one
    /// target" makes that reachable, and Karn, the Great Creator's `+1`
    /// activated with nothing to point at animated *Karn*, set his power and
    /// toughness to a mana value nobody had chosen, and the next state-based
    /// check swept the walker into the graveyard.
    pub targeted: bool,
    /// Whether this is a mana ability resolving off the stack (CR 605.3b).
    ///
    /// It changes what happens when the resolution *finishes*: there is no
    /// stack object to finalize, and its controller keeps priority.
    pub mana_ability: bool,
    /// The permanent whose ability an earlier effect of *this* resolution
    /// countered.
    ///
    /// Tishana's Tidebinder is one sentence in two effects — counter the
    /// ability, then strip the permanent it came from — and the second half
    /// has no way of its own to name that permanent: both read the same
    /// target, and an ability that has been countered ceases to exist
    /// (CR 701.6a), so the lookup finds nothing. It found nothing for as
    /// long as the card existed, and the rider had never once fired. The
    /// counter writes the answer down here on its way past instead, which
    /// is what makes the pair independent of the order the card lists them
    /// in.
    pub countered_source: Option<ObjectId>,
    /// What this resolution's targets looked like when it began.
    ///
    /// CR 608.2h: an effect that needs information about an object which is
    /// no longer in the zone it was expected to be in uses that object's
    /// *last known information*. A resolution expects its targets where they
    /// were when it started, so that is when the snapshot is taken — by
    /// [`run`], once, on the first pass, which is why the field is an
    /// `Option` and not an empty `Vec` that a re-entry after a player choice
    /// would refill from the wrong moment.
    ///
    /// It is `None` at every construction site for the same reason: nothing
    /// but `run` may decide when "the resolution began" was.
    pub target_lki: Option<Vec<TargetLki>>,
    /// Exact subject followed only through this resolution's own public-zone moves.
    pub subject: SubjectContext,
    /// Frozen effective words of this spell or ability.
    pub text: crate::text_changes::TextChangeMap,
    /// Where the effects of the second targeting mode of a spell cast with
    /// several modes begin (CR 700.2a, `progress::modal_program`), counted
    /// as the ops of the program still to run there: at that point
    /// `targets` becomes what that mode chose, the spell's second instance
    /// of the word "target". Its first mode's effects read the first
    /// instance up to there. `None` for everything else.
    ///
    /// Counted from the end and not as a program counter because the one
    /// thing that rewrites a program, a nested list that stops for a
    /// question ([`run_nested_with`]), replaces the op at the counter with
    /// what the nested list has left and moves everything after it; what
    /// is left after the point stays exactly what it was.
    ///
    /// `target_lki` includes both groups, so later instructions can still
    /// identify each targeted object's original incarnation and controller.
    pub retarget_left: Option<usize>,
    /// Connive/recruit state held while their draws may be replaced.
    pub loot: Option<Box<loot::Loot>>,
    /// Multi-stage coin contest, retained through life-gain draw replacements.
    pub coin_stake: Option<Box<coin_stakes::CoinStake>>,
    /// An instruction that does something to "the card drawn" whose draw
    /// had to wait for its replacements (`GameState::draws_to_offer`): how
    /// many cards had been drawn this turn and how many waiting draws had
    /// been replaced when it drew. It runs again once the draw is settled,
    /// and acts only on a card drawn plainly (CR 614.11b).
    pub draw_watch: Option<(usize, u32)>,
    /// Where "this way" (`Amount::ThisWay`) starts counting in the journal
    /// when an instruction narrowed it to one pass of a repeated process
    /// (`Effect::RepeatWhile`); `None` counts from the resolution's start.
    pub this_way_from: Option<usize>,
}

impl Resolution {
    /// The full resumable program and captured operands, excluding the pending
    /// operation whose payload is fingerprinted by its owning continuation.
    pub(crate) fn program_fingerprint(&self) -> u64 {
        let mut hash = crate::state::structural_fingerprint(&(
            (
                self.source,
                self.on_stack,
                self.controller,
                &self.effects,
                self.pc,
            ),
            (
                &self.targets,
                &self.second_targets,
                self.x,
                self.chosen_player,
                self.target_players,
            ),
            (
                self.event_object,
                self.event_mana.map(crate::trigger::EventMana::key),
                self.targeted,
                self.mana_ability,
                self.countered_source,
                self.text,
                self.retarget_left,
                &self.loot,
                self.draw_watch,
                self.this_way_from,
            ),
        ));
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(self.subject.fingerprint());
        if let Some(contest) = &self.coin_stake {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(contest));
        }
        if let Some(targets) = &self.target_lki {
            hash = hash.wrapping_mul(31).wrapping_add(1);
            for target in targets {
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(crate::state::structural_fingerprint(&(
                        target.id,
                        target.version,
                        target.controller,
                        crate::state::characteristics_fingerprint(&target.chars),
                    )));
            }
        }
        hash
    }
}

/// One target as it last existed where the resolution expected it.
///
/// The `version` is the discriminator and the whole of the fix: `version`
/// is bumped in exactly one place, `GameState::move_object`, so "the object
/// has a different version than when this resolution started" *is* "the
/// object is no longer in the zone the effect expected it in". A reader that
/// took the snapshot unconditionally would answer with stale information for
/// the opposite shape — an effect list that changes a permanent and then
/// reads it, still on the battlefield, which Inspirit Flagship Vessel does.
#[derive(Clone, Debug)]
pub struct TargetLki {
    /// The target this describes.
    pub id: ObjectId,
    /// Its `version` when the resolution began.
    pub version: u32,
    /// Controller before the target left its expected zone.
    pub controller: PlayerId,
    /// Its characteristics when the resolution began.
    pub chars: crate::object::Characteristics,
}

/// Identity and departure power retained for the triggering event's object.
fn event_object_identity(state: &GameState, res: &Resolution) -> Option<(u32, i16)> {
    if let Some((reference, power)) = res.subject.event_identity() {
        return Some((reference.version, power));
    }
    state
        .object(res.on_stack)?
        .riders
        .iter()
        .find_map(|r| match r {
            crate::object::Rider::EventObjectIdentity(version, power) => Some((*version, *power)),
            _ => None,
        })
}

/// The source incarnation an ability captured, independent of later moves.
pub(crate) fn source_version(state: &GameState, res: &Resolution) -> Option<u32> {
    if let Some(version) = res.subject.source_version {
        return Some(version);
    }
    state
        .object(res.on_stack)?
        .riders
        .iter()
        .find_map(|r| match r {
            crate::object::Rider::AbilitySourceVersion(version) => Some(*version),
            _ => None,
        })
        .or_else(|| {
            state
                .recorded_ability_source(res.on_stack)
                .map(|r| r.version)
        })
}

/// Identifies the card imposing a resolving mana payment, including last-known sources.
pub(crate) fn payment_for(state: &GameState, res: &Resolution) -> crate::casting::SpendFor {
    let source = source_version(state, res)
        .map(|version| baylee_core::ids::DamageSourceRef {
            object: res.source,
            version,
        })
        .or_else(|| state.source_identity(res.source));
    // The same test `Filter::HasCumulativeUpkeep` asks, so mana spendable
    // only on cumulative upkeep pays exactly the abilities that filter sees.
    if baylee_cards_dsl::Effect::is_cumulative_upkeep(&res.effects) {
        return crate::casting::SpendFor::CumulativeUpkeep(source);
    }
    source.map_or(
        crate::casting::SpendFor::Other,
        crate::casting::SpendFor::OtherFrom,
    )
}

/// Attachment captured when a waiting ability's source left the battlefield.
pub(crate) fn source_attachment_lki(
    state: &GameState,
    on_stack: ObjectId,
) -> Option<(ObjectId, u32)> {
    let riders = &state.object(on_stack)?.riders;
    let host = riders.iter().find_map(|rider| match rider {
        crate::object::Rider::SourceAttachmentLki(id) => Some(*id),
        _ => None,
    })?;
    let version = riders.iter().find_map(|rider| match rider {
        crate::object::Rider::SourceAttachmentVersion(version) => Some(*version),
        _ => None,
    })?;
    Some((host, version))
}

/// What an effect on [`Filter::This`](baylee_cards_dsl::Filter::This)
/// registers against: its exact current subject, including a public-zone
/// successor found by this resolution (CR 400.7j).
///
/// An ability that never said "target" is not removed when its object goes
/// (CR 115.1d; CR 608.2b checks targets only), so prowess on a token that
/// died in response still resolves. The token has ceased to exist
/// (CR 704.5d), and an effect that modifies characteristics fixes the
/// objects it affects as it begins (CR 611.2c): none. So it registers
/// nothing, as an ability that said "target" and got none does (#236).
///
/// Nor does it register against a phased-out permanent: a continuous effect
/// from a resolution leaves one out of its set, and "this includes
/// continuous effects that reference the permanent specifically"
/// (CR 702.26e).
pub(crate) fn this_to_affect(state: &GameState, res: &Resolution) -> Option<ObjectId> {
    subjects::this(state, res)
        .filter(|r| subjects::is_current(state, *r))
        .map(|r| r.object)
}

/// Whether `you` still control the permanent this resolution's ability came
/// from, as the object it was when the ability was put on the stack.
///
/// The second half is the stack object's own record: a source that left the
/// battlefield while its ability waited froze its power onto it
/// (`GameObject::source_power_lki`), and that record outlives a blink, which
/// brings back a new object (CR 400.7) under the same id.
/// Whether the source is the tapped permanent it was when the ability was
/// put on the stack, for a "for as long as this remains tapped" about to
/// begin (CR 611.2b).
fn source_still_tapped(state: &GameState, res: &Resolution) -> bool {
    let never_left = state
        .object(res.on_stack)
        .is_none_or(|o| o.source_power_lki.is_none());
    never_left
        && state.object(res.source).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && o.status.contains(crate::object::Status::TAPPED)
                && !o.status.contains(crate::object::Status::PHASED_OUT)
        })
}

fn source_still_yours(state: &GameState, res: &Resolution, you: PlayerId) -> bool {
    let never_left = state
        .object(res.on_stack)
        .is_none_or(|o| o.source_power_lki.is_none());
    never_left
        && state
            .object(res.source)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield && o.controller == you)
}

/// What earthbend makes of its land (CR 701.66a): "a 0/0 land creature
/// with haste in addition to its other types". Three modifiers in three
/// layers (4, 7b and 6), one effect each, sharing one timestamp.
static EARTHBEND_ANIMATION: [baylee_cards_dsl::Modifier; 3] = [
    baylee_cards_dsl::Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
    baylee_cards_dsl::Modifier::SetPT(0, 0),
    baylee_cards_dsl::Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HASTE),
];

/// Earthbend's delayed trigger (CR 701.66a): "return it to the battlefield
/// tapped under your control", "it" being the land that just died or was
/// exiled.
static EARTHBEND_RETURN: &[Effect] = &[Effect::ReturnToBattlefieldTapped {
    target: TargetSpec::EventObject,
}];

/// The first target's characteristics, as the effect asking is entitled to
/// see them (CR 608.2h).
///
/// Live while the object is still where the resolution left it, and the
/// snapshot [`run`] took once it is not. Swords to Plowshares is the card
/// that needs the second half: it exiles the creature and *then* reads its
/// power, and an object outside the battlefield reads its printed `base`,
/// because `move_object` clears the projection cache and the refresh pass
/// revisits only the battlefield and the stack. A Llanowar Elves with a
/// +1/+1 counter on it gained its controller one life instead of two — for
/// as long as the card has existed, the cache clear being older than every
/// entry that touches it.
fn target_chars<'a>(
    res: &'a Resolution,
    state: &'a GameState,
) -> Option<&'a crate::object::Characteristics> {
    let id = res.targets.first().copied()?;
    let obj = state.object(id);
    let known = res
        .target_lki
        .as_ref()
        .and_then(|lki| lki.iter().find(|l| l.id == id));
    match (obj, known) {
        // Still the same object in the same zone: nothing is lost by asking
        // it, and an effect list that changed it wants the change.
        (Some(obj), Some(known)) if obj.version == known.version => Some(obj.characteristics()),
        (_, Some(known)) => Some(&known.chars),
        (Some(obj), None) => Some(obj.characteristics()),
        (None, None) => None,
    }
}

impl Resolution {
    /// The exact text captured by this resolving spell or ability.
    pub(crate) fn rule_context(&self) -> crate::text_changes::RuleContext {
        crate::text_changes::RuleContext {
            source: self.source,
            text: self.text,
            resolved_name: self.subject.named.map(|(_, name)| name),
            // A relational filter's object: the first target, else the
            // triggering event's object (Glyph of Doom's delayed trigger
            // remembers its Wall as that).
            reference: self.targets.first().copied().or(self.event_object),
        }
    }
}

/// Amount evaluation with target context ([`Amount::TargetPower`]).
/// What a cost sacrificed, as `object`'s payment recorded it (CR 608.2h):
/// its mana value, power or toughness for the three `Sacrificed*` amounts,
/// 0 for none and for a negative power or toughness.
pub(crate) fn sacrificed_amount(object: Option<&GameObject>, amount: &Amount) -> u32 {
    let Some(lki) = object
        .and_then(|o| o.paid.as_ref())
        .and_then(|p| p.sacrificed_lki)
    else {
        return 0;
    };
    sacrificed_value(lki, amount)
}

fn sacrificed_value(lki: crate::object::SacrificedLki, amount: &Amount) -> u32 {
    match amount {
        Amount::SacrificedPower => u32::try_from(lki.power).unwrap_or(0),
        Amount::SacrificedToughness => u32::try_from(lki.toughness).unwrap_or(0),
        _ => lki.mana_value,
    }
}

#[allow(clippy::too_many_lines)] // one arm per resolution-only amount
pub(super) fn amount2(amount: &Amount, state: &GameState, you: PlayerId, res: &Resolution) -> u32 {
    match amount {
        Amount::CountersOnSource(kind) => state
            .damage_source(res.source, source_version(state, res))
            .map_or(0, |object| u32::from(object.counters.get(*kind))),
        Amount::SourcePower => state
            .damage_source(res.source, source_version(state, res))
            .and_then(|source| source.characteristics().power)
            .map_or(0, |power| power.max(0) as u32),
        Amount::EventPower => {
            let Some(source) = res.event_object else {
                return 0;
            };
            let identity = event_object_identity(state, res);
            let power = state
                .object(source)
                .filter(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && identity.is_none_or(|(v, _)| o.version == v)
                })
                .map(|o| o.characteristics().power.unwrap_or(0))
                .or_else(|| identity.map(|(_, p)| p))
                .or_else(|| {
                    state
                        .ltb_powers
                        .iter()
                        .find(|(id, _)| *id == source)
                        .map(|(_, p)| *p)
                })
                .unwrap_or(0);
            u32::try_from(power).unwrap_or(0)
        }
        Amount::EventLastToughness => state
            .object(res.on_stack)
            .and_then(|o| {
                o.riders.iter().find_map(|r| match r {
                    crate::object::Rider::EventDeparture(_, toughness) => {
                        Some((*toughness).max(0) as u32)
                    }
                    _ => None,
                })
            })
            .unwrap_or(0),
        Amount::TargetPower => target_chars(res, state)
            .and_then(|c| c.power)
            .map_or(0, |p| p.max(0) as u32),
        Amount::TargetToughness => target_chars(res, state)
            .and_then(|c| c.toughness)
            .map_or(0, |t| t.max(0) as u32),
        // Deliberately *not* through `target_chars`: the one card that reads
        // this is Reanimate, whose target is a creature card in a graveyard
        // and whose mana value is the printed one. A card that never was a
        // permanent has no last known information on the battlefield to
        // prefer.
        Amount::TargetCmc => res
            .targets
            .first()
            .and_then(|t| state.object(*t))
            .map_or(0, |o| {
                let chars = o.characteristics();
                chars
                    .front_mana_value
                    .map_or_else(|| chars.mana_cost.with_x(o.x_value).cmc(), u32::from)
            }),
        // Off the triggered ability, where stacking it wrote the event's
        // amount.
        Amount::EventManaValue => state
            .object(res.on_stack)
            .and_then(|object| {
                object.riders.iter().find_map(|rider| match rider {
                    crate::object::Rider::EventManaValue(value) => Some(*value),
                    _ => None,
                })
            })
            .unwrap_or(0),
        Amount::EventAmount => state
            .object(res.on_stack)
            .map_or(0, GameObject::event_amount),
        // Off the stack object, which is where the payment wrote it — a
        // spell's own, or the ability's rather than its permanent's.
        Amount::SacrificedManaValue | Amount::SacrificedPower | Amount::SacrificedToughness => {
            res.subject.sacrificed.map_or_else(
                || sacrificed_amount(state.object(res.on_stack), amount),
                |lki| sacrificed_value(lki, amount),
            )
        }
        Amount::ManaSpentToCast => state
            .object(res.on_stack)
            .and_then(|o| o.paid.as_ref())
            .map_or(0, |p| p.mana_spent),
        Amount::ManaTypeSpentOnX(color) => state
            .object(res.on_stack)
            .and_then(|o| o.paid.as_ref())
            .map_or(0, |p| p.mana_types_spent_on_x[color.index()]),
        Amount::SacrificedTotalPower => state
            .object(res.on_stack)
            .and_then(|o| o.paid.as_ref())
            .map_or(0, |p| {
                let total: i64 = p
                    .sacrificed_many
                    .iter()
                    .map(|(_, _, power)| i64::from(*power))
                    .sum();
                u32::try_from(total.max(0)).unwrap_or(u32::MAX)
            }),
        Amount::TappedPower => eval::tapped_power(state, res.on_stack),
        Amount::ThisWay(what) => this_way::count(state, res, *what, None),
        Amount::ThisWayBy { what, who } => players_of(*who, state, you, res)
            .into_iter()
            .map(|player| this_way::count(state, res, *what, Some(player)))
            .fold(0, u32::saturating_add),
        // A target is a new object in a graveyard since the resolution began
        // (its snapshot, CR 400.7): what this resolution put there.
        Amount::TargetsPutIntoGraveyard => {
            let Some(then) = res.target_lki.as_ref() else {
                return 0;
            };
            let moved = res.targets.iter().filter(|&&t| {
                let was = then.iter().find(|l| l.id == t).map(|l| l.version);
                state.object(t).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Graveyard && was.is_some_and(|v| v != o.version)
                })
            });
            u32::try_from(moved.count()).unwrap_or(u32::MAX)
        }
        // A wrapper around one of the above has to reach it through this
        // reader and not through `eval::amount`, which has no stack object.
        Amount::Plus { base, offset } => amount2(base, state, you, res).saturating_add(*offset),
        Amount::SaturatingSub { base, subtract } => {
            amount2(base, state, you, res).saturating_sub(*subtract)
        }
        Amount::Negated(base) => amount2(base, state, you, res),
        Amount::HandSize(who) => players_of(*who, state, you, res).first().map_or(0, |p| {
            u32::try_from(state.zones.list(ZoneLocation::Hand(*p)).len()).unwrap_or(u32::MAX)
        }),
        Amount::LifeTotal(who) => players_of(*who, state, you, res).first().map_or(0, |p| {
            u32::try_from(state.players[usize::from(p.get())].life).unwrap_or(0)
        }),
        Amount::ControlledCount { who, filter } => players_of(*who, state, you, res)
            .first()
            .map_or(0, |controller| {
                eval::count_controlled_by(state, *controller, you, filter, res.rule_context())
            }),
        Amount::LibrarySize(who) => players_of(*who, state, you, res).first().map_or(0, |p| {
            u32::try_from(state.zones.list(ZoneLocation::Library(*p)).len()).unwrap_or(u32::MAX)
        }),
        Amount::Half { base, round_up } => {
            let n = amount2(base, state, you, res);
            if *round_up { n.div_ceil(2) } else { n / 2 }
        }
        Amount::Divided {
            base,
            divisor,
            round_up,
        } => {
            let n = amount2(base, state, you, res);
            if *round_up {
                n.div_ceil(divisor.get())
            } else {
                n / divisor.get()
            }
        }
        Amount::Times { base, factor } => amount2(base, state, you, res).saturating_mul(*factor),
        Amount::Sum { left, right } => {
            amount2(left, state, you, res).saturating_add(amount2(right, state, you, res))
        }
        Amount::Difference { left, right } => {
            amount2(left, state, you, res).saturating_sub(amount2(right, state, you, res))
        }
        Amount::Players(who) => {
            u32::try_from(players_of(*who, state, you, res).len()).unwrap_or(u32::MAX)
        }
        Amount::CountOf {
            filter,
            zone: baylee_cards_dsl::ZoneSel::LinkedExile,
        } => u32::try_from(
            eval::linked_exile_cards(
                state,
                you,
                res.rule_context(),
                source_version(state, res),
                filter,
            )
            .len(),
        )
        .unwrap_or(u32::MAX),
        Amount::CountOf {
            filter,
            zone: baylee_cards_dsl::ZoneSel::GraveyardOf(who),
        } => players_of(*who, state, you, res)
            .into_iter()
            .map(|player| target_groups::graveyard_count(state, res, player, filter))
            .fold(0, u32::saturating_add),
        Amount::CountOf {
            filter,
            zone: baylee_cards_dsl::ZoneSel::HandChosen,
        } => {
            let context = res.rule_context();
            players_of(PlayerRel::Chosen, state, you, res)
                .into_iter()
                .flat_map(|p| state.zones.list(ZoneLocation::Hand(p)).iter().copied())
                .filter(|&id| {
                    state
                        .object(id)
                        .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
                })
                .count() as u32
        }
        other => eval::amount_with_context(other, state, you, res.rule_context(), res.x),
    }
}

/// [`amount2`] for an instruction applied to each player of a relation in
/// turn (`DrawCardsFor`, `DealDamage` to players, `LoseLife`, …), with
/// `player` the one it is applying to now: [`PlayerRel::Recipient`] inside
/// the amount names `player` ("the number of Islands that player controls",
/// "draws that many cards"), and every other amount reads as [`amount2`]
/// reads it.
pub(super) fn amount_for_player(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    res: &Resolution,
    player: PlayerId,
) -> u32 {
    let each = |inner: &Amount| amount_for_player(inner, state, you, res, player);
    match amount {
        Amount::CountOf {
            filter,
            zone: baylee_cards_dsl::ZoneSel::GraveyardOf(PlayerRel::Recipient),
        } => target_groups::graveyard_count(state, res, player, filter),
        Amount::ThisWayBy {
            what,
            who: PlayerRel::Recipient,
        } => this_way::count(state, res, *what, Some(player)),
        Amount::ControlledCount {
            who: PlayerRel::Recipient,
            filter,
        } => eval::count_controlled_by(state, player, you, filter, res.rule_context()),
        Amount::HandSize(PlayerRel::Recipient) => {
            u32::try_from(state.zones.list(ZoneLocation::Hand(player)).len()).unwrap_or(u32::MAX)
        }
        Amount::LifeTotal(PlayerRel::Recipient) => {
            u32::try_from(state.players[usize::from(player.get())].life).unwrap_or(0)
        }
        Amount::LibrarySize(PlayerRel::Recipient) => {
            u32::try_from(state.zones.list(ZoneLocation::Library(player)).len()).unwrap_or(u32::MAX)
        }
        Amount::Plus { base, offset } => each(base).saturating_add(*offset),
        Amount::SaturatingSub { base, subtract } => each(base).saturating_sub(*subtract),
        Amount::ConstantMinus { constant, subtract } => constant.saturating_sub(each(subtract)),
        Amount::Negated(base) => each(base),
        Amount::Half { base, round_up } => {
            let n = each(base);
            if *round_up { n.div_ceil(2) } else { n / 2 }
        }
        Amount::Divided {
            base,
            divisor,
            round_up,
        } => {
            let n = each(base);
            if *round_up {
                n.div_ceil(divisor.get())
            } else {
                n / divisor.get()
            }
        }
        Amount::Times { base, factor } => each(base).saturating_mul(*factor),
        Amount::Sum { left, right } => each(left).saturating_add(each(right)),
        Amount::Difference { left, right } => each(left).saturating_sub(each(right)),
        other => amount2(other, state, you, res),
    }
}

/// The filters a continuous effect created by a **resolution** is registered
/// with (CR 611.2c).
///
/// A static ability's effect is dynamic — an anthem lifts a creature that
/// enters ten turns later, because the ability keeps applying for as long as
/// its source is there. A spell or ability that *resolves* is the opposite:
/// it happens once, and if what it does is modify characteristics or change
/// control, the objects it affects are the ones it found. So the filter is
/// read here, at the moment the effect begins, and the effect is registered
/// against the objects it named rather than against the question.
///
/// Filters that explicitly name another zone enumerate the object arena;
/// ordinary battlefield effects retain the cheaper battlefield-only walk.
///
/// One effect per object rather than one effect naming several, because an
/// [`EffectFilter`](crate::effects::EffectFilter) names exactly one — the
/// same shape `Effect::PumpTarget` already registers for a spell with two
/// targets.
/// `only` narrows the set to the seats it lists, which is the half no
/// `Filter` can do: "creatures **target player** controls" depends on a
/// choice, and a filter is told the ability's controller and its source and
/// nothing else. The seat is known here, so the set is bound here — which is
/// where CR 611.2c wants it bound anyway. A narrowed effect that could not
/// lock would have nowhere to put the seat, and no card asks for one: only
/// `Effect::PumpFilter` passes `only`, and `Modifier::ModifyPT` always
/// locks.
#[cfg(test)]
pub(super) fn bound_now(
    state: &GameState,
    filter: &'static baylee_cards_dsl::Filter,
    modifier: &baylee_cards_dsl::Modifier,
    you: PlayerId,
    this: ObjectId,
    only: Option<&[PlayerId]>,
) -> SmallVec<[crate::effects::EffectFilter; 4]> {
    bound_now_with_context(
        state,
        filter,
        modifier,
        you,
        eval::live_context(state, this),
        only,
    )
}

pub(super) fn bound_now_with_context(
    state: &GameState,
    filter: &'static baylee_cards_dsl::Filter,
    modifier: &baylee_cards_dsl::Modifier,
    you: PlayerId,
    context: crate::text_changes::RuleContext,
    only: Option<&[PlayerId]>,
) -> SmallVec<[crate::effects::EffectFilter; 4]> {
    // A filter that names the resolution's reference object (Feint's "that
    // creature and each creature blocking it"): the reference is gone once
    // the resolution is, so an effect that does not lock its set (CR 611.2c)
    // keeps the object it is about, and one that does is bound now.
    if !crate::effects::locks_its_set(modifier) && crate::state::filter_reads_reference(filter) {
        debug_assert!(
            only.is_none(),
            "a dynamic effect cannot carry the seat its card named",
        );
        return context
            .reference
            .and_then(|id| state.object(id).map(|o| (id, o.version)))
            .map(|(id, version)| crate::effects::EffectFilter::DslAbout(filter, id, version))
            .into_iter()
            .collect();
    }
    if !crate::effects::locks_its_set(modifier) {
        return only.map_or_else(
            || smallvec::smallvec![crate::effects::EffectFilter::Dsl(filter)],
            |seats| {
                smallvec::smallvec![crate::effects::EffectFilter::DslControlledBy(
                    filter,
                    seats.iter().copied().collect(),
                )]
            },
        );
    }
    let matches = |o: &crate::object::GameObject| {
        !o.status.contains(crate::object::Status::PHASED_OUT)
            && only.is_none_or(|seats| seats.contains(&o.controller))
            && eval::matches_with_context(filter, state, o, you, context)
    };
    if crate::state::filter_reaches_other_zones(filter) {
        state
            .arena
            .iter()
            .filter(|(_, o)| matches(o))
            .map(|(id, _)| crate::effects::EffectFilter::object(state, id))
            .collect()
    } else {
        state
            .battlefield_seen()
            .filter(|id| state.object(*id).is_some_and(matches))
            .map(|id| crate::effects::EffectFilter::object(state, id))
            .collect()
    }
}

/// The seats a [`PlayerRel`] names *during a resolution*.
///
/// [`eval::players`] answers the half that the state alone can answer, and
/// deliberately answers `None` for the two relations that need the
/// resolution's own context: `Chosen` is the player this spell or ability
/// targeted, `ControllerOfTarget` is read off its first object target. An
/// effect that reaches for `eval::players` directly therefore does *nothing*
/// on a card that says "target opponent" — which is exactly how Abraded
/// Bluffs shipped as a land that deals no damage. **Every effect inside a
/// resolution asks this function**, whatever relation the card names; the
/// `other` arm below is the only place in the module that may unwrap the
/// state-only half, and its `expect` is unreachable because the two context
/// relations are matched above it.
pub(super) fn players_of(
    rel: PlayerRel,
    state: &GameState,
    you: PlayerId,
    res: &Resolution,
) -> Vec<PlayerId> {
    match rel {
        PlayerRel::OwnerOfSource => state
            .damage_source(res.source, source_version(state, res))
            .map(|source| source.owner)
            .filter(|p| !state.has_left(*p))
            .into_iter()
            .collect(),
        PlayerRel::Chosen => res.chosen_player.into_iter().collect(),
        // Only an instruction applying to each player names one, and it
        // reads its amounts through `amount_for_player`.
        PlayerRel::Recipient => Vec::new(),
        PlayerRel::SecondTargetPlayerOrController => target_groups::second_player(state, res)
            .into_iter()
            .collect(),
        // A card has its owner wherever it is (CR 108.3).
        PlayerRel::OwnerOfEvent => res
            .event_object
            .and_then(|id| state.object_or_departed(id))
            .map(|o| o.owner)
            .filter(|p| !state.has_left(*p))
            .into_iter()
            .collect(),
        // Of the triggering attacker, or the source for a nontriggered instruction (CR 508.5).
        PlayerRel::DefendingPlayer => {
            eval::defending_player_of(state, res.event_object.unwrap_or(res.source), you)
                .into_iter()
                .collect()
        }
        // Last known first (CR 608.2h), for the same reason as the event's
        // below: "Exile target creature. Its controller gains life …" reads
        // its second sentence after the first has moved the creature, and
        // nothing controls a card in exile.
        PlayerRel::TargetPlayerOrController => res
            .target_players
            .iter()
            .next()
            .or(res.chosen_player)
            .map_or_else(
                || players_of(PlayerRel::ControllerOfTarget, state, you, res),
                |player| vec![player],
            ),
        PlayerRel::ControllerOfTarget => res
            .targets
            .first()
            .and_then(|t| {
                res.target_lki
                    .as_ref()
                    .and_then(|all| {
                        all.iter().find(|lki| {
                            lki.id == *t
                                && state.object(*t).is_none_or(|o| o.version != lki.version)
                        })
                    })
                    .map(|lki| lki.controller)
                    .or_else(|| state.last_known_controller(*t))
            })
            .into_iter()
            .collect(),
        // Last known first (CR 603.10a): the object left the battlefield,
        // and what it is now — a card in a graveyard, or no object at all —
        // is controlled by nobody. An event about a permanent that is still
        // there (an enter trigger) has no look-back entry and answers with
        // the controller it has now. A player who has since left the game
        // is nobody's "that player" (CR 800.4a).
        PlayerRel::ControllerOfEvent => state
            .object(res.on_stack)
            .and_then(|o| {
                o.riders.iter().find_map(|r| match r {
                    crate::object::Rider::EventDeparture(controller, _) => Some(*controller),
                    _ => None,
                })
            })
            .or_else(|| {
                res.event_object.and_then(|id| {
                    if let Some((version, _)) = event_object_identity(state, res) {
                        state
                            .damage_source(id, Some(version))
                            .map(|object| object.controller)
                    } else {
                        state.last_known_controller(id)
                    }
                })
            })
            .filter(|seat| !state.has_left(*seat))
            .into_iter()
            .collect(),
        // Off the triggered ability, where stacking it wrote the player the
        // event was about — who drew, who cast, the one it dealt damage to
        // — as `Amount::EventAmount` reads the amount. One seat, never each
        // opponent: an ability with no such event names nobody.
        PlayerRel::EventPlayer | PlayerRel::DamagedPlayer => res
            .subject
            .event_player()
            .or_else(|| {
                state.object(res.on_stack).and_then(|o| {
                    o.riders.iter().find_map(|r| match r {
                        crate::object::Rider::EventPlayer(seat) => Some(*seat),
                        _ => None,
                    })
                })
            })
            .filter(|seat| !state.has_left(*seat))
            .into_iter()
            .collect(),
        // "Enchanted land's controller" (CR 303.4e): whoever controls what
        // the source is attached to now. An Aura that has left, or that
        // enchants nothing, names nobody.
        PlayerRel::ControllerOfAttached => eval::controller_of_attached(state, res.source)
            .into_iter()
            .collect(),
        other => eval::players(other, state, you)
            .expect("the context relations are matched above this arm"),
    }
}

/// Flattens nested `Sequence`s into one flat op list.
#[must_use]
pub fn flatten(effects: &'static [Effect]) -> Vec<Effect> {
    fn go(e: &Effect, out: &mut Vec<Effect>) {
        match e {
            Effect::Sequence(parts) => {
                for p in *parts {
                    go(p, out);
                }
            }
            other => out.push(*other),
        }
    }
    let mut out = Vec::new();
    for e in effects {
        go(e, &mut out);
    }
    out
}

/// What the resolution machine produced.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // The result moves a choice directly into the driver; boxing adds an allocation at every suspension.
pub enum Flow {
    /// All operations are done.
    Complete,
    /// Settle entry modifiers and their following actions without granting priority.
    Enter,
    /// Suspended: a choice is required (pending is set by the caller).
    Wait(Pending),
}

/// Runs a resolution until it completes or suspends on a choice.
#[must_use]
pub fn run(state: &mut GameState, res: &mut Resolution) -> Flow {
    subjects::flush(state, res);
    linked_reanimation::flush(state, res);
    // A payment may have moved several cards just before this resolution
    // began (including a mana ability). Settle that order before any effect
    // can read the graveyard.
    if let Some(pending) = order_before_continuing(state, res, None) {
        return pending;
    }
    // The moment CR 608.2h measures from. `run` is re-entered after every
    // suspended choice, so this has to be the *first* entry and not any
    // entry, which is what the `Option` says.
    if res.target_lki.is_none() {
        if !res.mana_ability {
            let index = state
                .object(res.on_stack)
                .and_then(|object| object.ability)
                .map_or(0, |loc| loc.index);
            res.text = state.ability_text(res.on_stack, index);
        }
        res.target_lki = Some(
            res.targets
                .iter()
                .chain(res.second_targets.iter())
                .filter_map(|&id| {
                    state.object(id).map(|o| TargetLki {
                        id,
                        version: o.version,
                        controller: o.controller,
                        chars: o.characteristics().clone(),
                    })
                })
                .collect(),
        );
    }
    loop {
        if let Some(pending) = crate::replacement::counter_order::pending(state) {
            res.awaiting = Some(AwaitingOp::CounterPlacementChoice);
            return Flow::Wait(pending);
        }
        if let Some(pending) = crate::sba::destruction_choice(state) {
            res.awaiting = Some(match pending {
                Pending::YesNo {
                    prompt: YesNoPrompt::UseShieldCounter { .. },
                    ..
                } => AwaitingOp::ShieldCounterChoice,
                Pending::YesNo {
                    prompt: YesNoPrompt::UseDamageClearingReplacement { .. },
                    ..
                } => AwaitingOp::DamageClearingChoice,
                _ => AwaitingOp::RegenerationChoice,
            });
            return Flow::Wait(pending);
        }
        // A draw Island Sanctuary may replace, queued by the instruction
        // before (or by an answer this resolution just took), is asked
        // about before the next one runs (CR 614.11a).
        if let Some(pending) = ask_offered_draw(state, res) {
            return Flow::Wait(pending);
        }
        if res.pc >= res.effects.len() {
            break;
        }
        if state.numeric_failure.is_some() {
            return Flow::Complete;
        }
        // The second targeting mode of a spell cast with several begins
        // here: "target" means what that mode chose from now on.
        if res.retarget_left == Some(res.effects.len() - res.pc) {
            res.retarget_left = None;
            res.target_players = res.subject.second_players();
            res.chosen_player = (res.target_players.len() == 1)
                .then(|| res.target_players.iter().next())
                .flatten();
            res.targets = std::mem::take(&mut res.second_targets)
                .into_iter()
                .collect();
        }
        let op = res.effects[res.pc];
        // Continuous effects apply at all times (CR 613), so an effect of
        // this resolution sees what the one before it did: Bridgeworks
        // Battle's "gets +2/+2 until end of turn. It fights …" is dealt at
        // the pumped power. The projection used to be refreshed only between
        // engine steps, which made every effect after a pump or a counter in
        // the same resolution read the creature as it had been before it.
        // One generation compare when nothing moved, which is every effect
        // that did not just change a characteristic.
        state.capture_entry_history();
        state.refresh_characteristics();
        state.award_enduring_stories();
        state.award_citys_blessings();
        // Each instruction is an event of its own: a source that left in an
        // earlier one applies none of its rules to this one (#291). Not
        // after `exec`, which may have suspended in the middle of one.
        crate::replacement::expire_departed_rules(state);
        let since = state.journal.last_seq();
        state.entries.begin_event(since, Some(res.controller));
        let pending = exec(state, res, op);
        linked_reanimation::flush(state, res);
        crate::graveyard_order::capture(state, since);
        if pending.is_none() {
            advance_program(res);
            if matches!(res.awaiting, Some(AwaitingOp::Entry { .. })) {
                return Flow::Enter;
            }
        }
        if let Some(flow) = order_before_continuing(state, res, pending) {
            return flow;
        }
    }
    // The resolving object's own departure is the next event.
    crate::replacement::expire_departed_rules(state);
    Flow::Complete
}

/// Preserve an instruction's own choice while CR 404.3 is answered. The
/// program counter already names the next instruction when `next` is absent.
/// The question about the first waiting draw Island Sanctuary could replace
/// (`GameState::draws_to_offer`), once every draw ahead of it is made.
///
/// Never while another question of this resolution is out, which would be
/// overwritten, and never inside a mana ability, which asks nothing it does
/// not print: the machine asks those (`Engine::offer_queued_draw`).
fn ask_offered_draw(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    if res.awaiting.is_some() || res.mana_ability || state.draws_to_offer.is_empty() {
        return None;
    }
    let (player, source) = state.next_draw_offer()?;
    res.awaiting = Some(AwaitingOp::SkipDraw { source });
    Some(state.draw_offer_question(player, source))
}

fn order_before_continuing(
    state: &mut GameState,
    res: &mut Resolution,
    next: Option<Pending>,
) -> Option<Flow> {
    if matches!(res.awaiting, Some(AwaitingOp::Entry { .. })) {
        return next.map(Flow::Wait);
    }
    let entry_choice = crate::entry::advance(state);
    if entry_choice.is_some() || state.entries.has_after() {
        let next = next.map(|pending| {
            Box::new((
                res.awaiting.take().expect("instruction continuation"),
                pending,
            ))
        });
        res.awaiting = Some(AwaitingOp::Entry { next, after: None });
        return Some(entry_choice.map_or(Flow::Enter, Flow::Wait));
    }
    // A nested program already suspended on this very queue.
    if matches!(res.awaiting, Some(AwaitingOp::GraveyardOrder { .. })) {
        return next.map(Flow::Wait);
    }
    let Some(question) = crate::graveyard_order::pending(state) else {
        return next.map(Flow::Wait);
    };
    let next =
        next.map(|pending| Box::new((res.awaiting.take().expect("choice continuation"), pending)));
    res.awaiting = Some(AwaitingOp::GraveyardOrder { next });
    Some(Flow::Wait(question))
}

/// Continue the original instruction after every entry replacement has finished.
pub(crate) fn resume_entries(state: &mut GameState, res: &mut Resolution) -> Flow {
    linked_reanimation::flush(state, res);
    let Some(AwaitingOp::Entry { next, after }) = res.awaiting.take() else {
        unreachable!("entry continuation")
    };
    if let Some(after) = after {
        debug_assert!(next.is_none(), "post-entry work owns its next decision");
        if let Some(pending) = after_entry::resume(state, res, after) {
            return Flow::Wait(pending);
        }
        advance_program(res);
        return run(state, res);
    }
    if let Some(next) = next {
        let (awaiting, pending) = *next;
        res.awaiting = Some(awaiting);
        Flow::Wait(pending)
    } else {
        run(state, res)
    }
}

/// Counted copies retain one instruction across every target-choice suspension.
fn advance_program(res: &mut Resolution) {
    if res.loot.is_none()
        && res.coin_stake.is_none()
        && res.draw_watch.is_none()
        && !matches!(res.awaiting, Some(AwaitingOp::Entry { after: Some(_), .. }))
        && !matches!(res.effects.get(res.pc), Some(Effect::CopyThisSpell { amount: Amount::Fixed(n) }) if *n > 0)
    {
        res.pc += 1;
    }
}

fn finish_choice(state: &mut GameState, res: &mut Resolution, since: u64) -> Flow {
    crate::graveyard_order::capture(state, since);
    advance_program(res);
    match order_before_continuing(state, res, None) {
        Some(flow) => flow,
        None => run(state, res),
    }
}

fn next_choice(state: &mut GameState, res: &mut Resolution, since: u64, pending: Pending) -> Flow {
    crate::graveyard_order::capture(state, since);
    order_before_continuing(state, res, Some(pending)).expect("next choice")
}

/// Sylvan Library's second question: which of `cards` go back on top of
/// the library, the rest paid for with `life` each. Everything the life
/// total cannot cover has to go back (CR 119.4), which is the minimum.
fn put_back_question(
    state: &GameState,
    res: &mut Resolution,
    cards: Vec<ObjectId>,
    life: u16,
) -> Option<Pending> {
    if cards.is_empty() {
        return None;
    }
    let you = res.controller;
    let n = u8::try_from(cards.len()).unwrap_or(u8::MAX);
    let payable = if life == 0 {
        u32::from(n)
    } else {
        u32::try_from(state.life_payable(you)).unwrap_or(0) / u32::from(life)
    };
    let must_go_back = u32::from(n).saturating_sub(payable);
    res.awaiting = Some(AwaitingOp::PayOrPutBack {
        cards: cards.clone(),
        life,
    });
    Some(Pending::ChooseCards {
        player: you,
        options: cards,
        min: u8::try_from(must_go_back).unwrap_or(n),
        max: n,
        prompt: ChoicePrompt::PutBackOnTop,
        total: None,
    })
}

/// Whether `owner` stands in `rel` to `you`: "a card an opponent owns".
fn owner_is(state: &GameState, rel: PlayerRel, owner: PlayerId, you: PlayerId) -> bool {
    match rel {
        PlayerRel::You => owner == you,
        PlayerRel::Opponent | PlayerRel::EachOpponent => state.is_opponent(owner, you),
        PlayerRel::EachPlayer => true,
        _ => false,
    }
}

/// Executes one operation; returns `Some(pending)` when it suspends.
fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let before = subjects::before(state, res);
    let next_effect = state.effects.hashed_parts().2;
    let op = res.text.effect_immediate(op);
    let pending = match op {
        Effect::SearchLibrary { .. }
        | Effect::SearchLibraryBeforeShuffle { .. }
        | Effect::SearchSharedLandType
        | Effect::SearchWithManaDifference { .. }
        | Effect::TargetedGraveyardToTop
        | Effect::EachPlayerSearchBeforeShuffle { .. }
        | Effect::Scry { .. }
        | Effect::Surveil { .. }
        | Effect::Explore { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::PutFromHandOntoBattlefield { .. }
        | Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::SearchOpponentSplits { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayThen { .. }
        | Effect::PlayerMayPayThenElse { .. }
        | Effect::PlayerMayPayManaEachThenElse { .. }
        | Effect::UntapChosenPayingEach { .. }
        | Effect::PlayerMayPayManaOr { .. }
        | Effect::PlayerMayPayManaThen { .. }
        | Effect::MayPayEnergyOr { .. }
        | Effect::PlayerMayPayLifeOr { .. }
        | Effect::PlayerMayGetPoisonOr { .. }
        | Effect::MayExileFromYourGraveyardThen { .. }
        | Effect::ReturnChosenLinkedToBattlefield { .. }
        | Effect::MayRemoveCountersSelfCreateTokens { .. }
        | Effect::MayExileTokensCreatedWithThisForCounters { .. }
        | Effect::EachPlayerMayPutFromHandOntoBattlefieldRepeatUntilNone { .. }
        | Effect::PlayerMayMillOr { .. }
        | Effect::RaidingParty
        | Effect::ChoosePhaseToSkipThisTurn
        | Effect::ChooseOne { .. }
        | Effect::MayBehold { .. }
        | Effect::ChooseCardName { .. }
        | Effect::GuessChosenNameInHand { .. }
        | Effect::RevealHandForGuess
        | Effect::DrawIfGuessWrong
        | Effect::ChooseNumber { .. }
        | Effect::DestroyWeakest { .. }
        | Effect::ExchangeGreatestManaValue { .. }
        | Effect::PlayerMayPayCostThen { .. }
        | Effect::PlayerMaySacrificeNOr { .. }
        | Effect::PlayerMaySacrificeNThen { .. }
        | Effect::SacrificeAndIf { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::PayManaToPreventDamage { .. }
        | Effect::SacrificeChosenByOpponent { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. }
        | Effect::AddManaUnlessAnyPlayerPays { .. }
        | Effect::AddMana { .. }
        | Effect::MayDo { .. }
        | Effect::MayDoFor { .. }
        | Effect::MayDoOnceEachTurn { .. }
        | Effect::OwnerPutsOnTopOrBottom { .. }
        | Effect::PayLifeOrEnterTapped { .. } => exec_choice(state, res, op),
        _ => exec_immediate(state, res, op),
    };
    subjects::after(state, res, &before);
    if state.effects.hashed_parts().2 > next_effect {
        let created: smallvec::SmallVec<[baylee_core::ids::EffectId; 4]> = state
            .effects
            .iter()
            .filter(|effect| {
                effect.id.get() >= next_effect
                    && effect.origin == crate::effects::EffectOrigin::Resolution
            })
            .map(|effect| effect.id)
            .collect();
        for id in created {
            if !state
                .effect_text_overrides
                .iter()
                .any(|(known, _)| *known == id)
            {
                state
                    .effect_text_overrides
                    .push((id, crate::text_changes::TextOrigin::Frozen(res.text)));
            }
        }
    }
    pending
}

/// The card ability a resolution belongs to.
///
/// This is the handle a seat's standing answer is stored under and the
/// label a client puts on a stack entry, so it has to name the *ability*
/// (Ondu Cleric's rally trigger) rather than only the permanent.
#[must_use]
pub fn resolving_ability(
    state: &GameState,
    res: &Resolution,
) -> Option<baylee_core::ids::AbilityRef> {
    use baylee_core::ids::AbilityRef;
    let obj = state.object(res.on_stack)?;
    if let Some(loc) = obj.ability {
        // No card, no handle: a token's, a token copy's and an emblem's
        // ability is addressed by nothing a standing answer could be filed
        // under, and saying so is the whole point of the `Option`. It used
        // to answer with card index 0, so one "always say yes" would have
        // covered every such ability at once — and a real card besides.
        return state
            .printed_ability_list(obj.id)
            .and_then(|list| list.entry(loc.index as usize))
            .and_then(|entry| entry.provenance.ability_ref())
            .or_else(|| loc.card.map(|card| AbilityRef::new(card, loc.index)));
    }
    // A spell resolving: what it does is its spell ability, which is not
    // an entry in the card's ability list.
    obj.card
        .map(|c| AbilityRef::new(c.index, AbilityRef::SPELL))
}

/// Whether an optional clause can still be done, asked before it is offered.
///
/// CR 608.2d: "The player can't choose an option that's illegal or
/// impossible." "You may sacrifice this land" after the land has gone is
/// that, and so is "you may put that card onto the battlefield" once the
/// card has left the graveyard (CR 400.7). Neither is asked, and the clause
/// does not happen; for "do this only once each turn" that also keeps the
/// turn's one go. Only these two bodies are read, each with the predicate
/// its own effect checks, and any other "may" is asked as before.
fn may_clause_possible(state: &GameState, res: &Resolution, effects: &[Effect]) -> bool {
    match effects {
        // The head of the list, not the whole of it: "you may sacrifice
        // this. If you do, …" is the cost and then what it buys (CR 118.12),
        // and a yes that cannot pay must not buy the rest. Safe Haven
        // destroyed in response to its upkeep trigger returned everything
        // exiled with it while the list was matched whole.
        [Effect::SacrificeSelf, ..] => zones::can_sacrifice_self(state, res),
        [Effect::RemoveAllCountersSelf { .. }, ..] => subjects::source(state, res)
            .is_some_and(|source| subjects::on_battlefield(state, source)),
        [Effect::RemoveCounterSelf { kind, n }, ..] => state.object(res.source).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && !o.status.contains(crate::object::Status::PHASED_OUT)
                && o.counters.get(*kind) >= *n
        }),
        [
            Effect::GraveyardToBattlefield {
                target: TargetSpec::EventObject,
                ..
            },
        ] => res.event_object.is_some_and(|card| {
            state
                .object(card)
                .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard)
        }),
        _ => true,
    }
}

/// Runs a nested branch (If*/kicked-style conditional effects) inline;
/// a suspension inside the branch splices its remaining ops into the
/// parent's program and propagates the choice.
///
/// The splice *replaces* the parent's op and starts at the nested program
/// counter, and both halves of that are load-bearing. Inserting in front of
/// the parent's op instead left the branch itself standing in the program,
/// so resuming ran back into it and asked the same question a second time —
/// Luminarch Ascension offered its quest counter twice and took two. And
/// splicing the whole nested program rather than its tail would re-run the
/// ops the branch had already finished before it suspended. Neither could
/// be seen until an effect that suspends was put inside a branch, which is
/// what [`Effect::MayDo`] did.
fn run_nested_with(
    state: &mut GameState,
    res: &mut Resolution,
    effects: Vec<Effect>,
    targets: SmallVec<[ObjectId; 2]>,
) -> Option<Pending> {
    let mut nested = Resolution {
        source: res.source,
        on_stack: res.on_stack,
        controller: res.controller,
        effects,
        pc: 0,
        targets,
        second_targets: res.second_targets.clone(),
        event_object: res.event_object,
        x: res.x,
        chosen_player: res.chosen_player,
        target_players: res.target_players,
        targeted: res.targeted,
        awaiting: None,
        mana_ability: false,
        countered_source: res.countered_source,
        target_lki: None,
        subject: res.subject.clone(),
        text: res.text,
        event_mana: res.event_mana,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: res.this_way_from,
        coin_stake: None,
    };
    let flow = run(state, &mut nested);
    res.subject = nested.subject.clone();
    match flow {
        Flow::Complete => None,
        Flow::Enter => {
            res.awaiting = nested.awaiting;
            res.loot = nested.loot;
            res.coin_stake = nested.coin_stake;
            res.draw_watch = nested.draw_watch;
            let tail = nested.effects.split_off(nested.pc);
            // No public question was emitted, so the outer instruction still
            // advances once before this inserted continuation is resumed.
            let insert = res.pc + 1;
            res.effects.splice(insert..insert, tail);
            None
        }
        Flow::Wait(pending) => {
            res.awaiting = nested.awaiting;
            res.loot = nested.loot;
            res.coin_stake = nested.coin_stake;
            res.draw_watch = nested.draw_watch;
            let tail = nested.effects.split_off(nested.pc);
            res.effects.splice(res.pc..=res.pc, tail);
            Some(pending)
        }
    }
}

/// Runs a nested static branch inline (see [`run_nested_with`]).
fn run_nested(
    state: &mut GameState,
    res: &mut Resolution,
    branch: &'static [Effect],
) -> Option<Pending> {
    let targets = res.targets.clone();
    run_nested_with(state, res, flatten(branch), targets)
}

/// Sacrifices `victims` as one event: every departure is read before any of
/// them leaves, so a creature and the Aura on it go together and each
/// leave trigger sees the board as it was (Lich's rulings).
pub(super) fn sacrifice_together(state: &mut GameState, victims: &[ObjectId]) {
    let departures: Vec<_> = victims
        .iter()
        .map(|&id| state.departure_snapshot(id))
        .collect();
    for (&victim, departure) in victims.iter().zip(departures) {
        let Some(_owner) = state
            .object(victim)
            .filter(|o| o.zone == crate::zone::Zone::Battlefield)
            .map(|o| o.owner)
        else {
            continue;
        };
        if let Some(obj) = state.object_mut(victim) {
            obj.kind = ObjectKind::Card;
        }
        let _ = state.sacrifice_object_with_departure(victim, Cause::Effect, departure);
    }
}

/// "Sacrifice that many … If you can't, you lose the game": `amount`
/// matching permanents are chosen by their controller and sacrificed
/// together. Holding no more than that, there is nothing to choose, and
/// holding fewer, the player loses as well (CR 104.3e).
fn sacrifice_amount_or_lose(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    amount: &Amount,
) -> Option<Pending> {
    let you = res.controller;
    let n = amount2(amount, state, you, res) as usize;
    if n == 0 {
        return None;
    }
    let options = chosen::options(state, you, filter, you, res.source);
    if options.len() > n {
        let n = u8::try_from(n).unwrap_or(u8::MAX);
        res.awaiting = Some(AwaitingOp::SacrificeAllChosen);
        return Some(Pending::ChooseCards {
            player: you,
            options,
            min: n,
            max: n,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    sacrifice_together(state, &options);
    if options.len() < n {
        let _ = sba::lose_by_effect(state, you);
    }
    None
}

/// Untaps one permanent and says so.
///
/// One door for both untap **effects**, and it exists because the two used
/// to disagree with the third. The untap step journals
/// `GameEvent::ObjectUntapped` (CR 502.3); `Effect::UntapTarget` wrote the
/// bit and journalled nothing, so an untap from an effect was invisible to
/// anything reading the game's own record of what happened — a replay, a
/// client's log, and any "becomes untapped" trigger the DSL might learn.
///
/// That was latent rather than broken: `AbilityDef::Trigger` has
/// `BecomesTapped` and no untapped twin, so nothing could have been
/// listening. Latent is the state a rule is in just before somebody adds the
/// trigger and cannot work out why it never fires, so the fix comes with the
/// second effect rather than after it, and
/// `card_tests::lands::deserted_temple_says_so_when_it_untaps_a_land` is
/// what holds it — a test on the *older* variant, because that is the one
/// that was wrong.
///
/// A permanent that is already untapped is left alone and journals nothing:
/// untapping an untapped permanent is not an event, and recording one would
/// put a "became untapped" in the log for a permanent that did not.
fn untap(state: &mut GameState, id: ObjectId) {
    if state.set_tapped(id, false) {
        state.journal.record(GameEvent::ObjectUntapped {
            object: id,
            cause: Cause::Effect,
        });
    }
}

/// Record only an actual tap; entering tapped is handled by the entry pipeline.
fn tap(state: &mut GameState, id: ObjectId) -> bool {
    if !state.set_tapped(id, true) {
        return false;
    }
    state.journal.record(GameEvent::ObjectTapped {
        object: id,
        cause: Cause::Effect,
    });
    true
}

/// Each `(object, player)`: the player gains control of the object for as
/// long as it stays where it is, all at once.
///
/// A layer-2 effect (CR 613.1b) and not a new default controller. The
/// difference is the whole of CR 800.4: when a player leaves the game the
/// effects giving them control end and the object goes back to whoever
/// controls it without them (a Gilded Drake'd creature to its owner), while
/// what they control by default is exiled (a creature they reanimated out
/// of someone else's graveyard). `base_controller` is that default, so it
/// is never written here. The effects keep their timestamps, so a later
/// taker wins and an earlier one's control comes back when the later one
/// leaves (CR 613.7).
///
/// Nothing is registered for a player who has left the game (CR 800.4b), or
/// for a player who controls the object by default when no effect gives it
/// to anybody: that effect would change nothing, now or later. One that
/// does change nothing *now* is still registered, because it outlives the
/// shorter effect that hides it: a creature stolen back until end of turn
/// is its thief's again after the cleanup step.
///
/// Summoning sickness restarts where control moved (CR 302.6), as the
/// projection moves it; the journal says who has each object now.
pub(super) fn gain_control(state: &mut GameState, changes: &[(ObjectId, PlayerId)]) {
    let timestamp = state.next_timestamp();
    for &(id, player) in changes {
        if state.has_left(player) || !crate::permanent_guards::can_gain_control(state, id, player) {
            continue;
        }
        let Some(obj) = state.object(id) else {
            continue;
        };
        let held = state.effects.iter().any(|fx| {
            fx.modifier == baylee_cards_dsl::Modifier::GainControl
                && crate::effects::applies_to(state, fx, obj)
        });
        if !held && obj.base_controller == player {
            continue;
        }
        let filter = crate::effects::EffectFilter::object(state, id);
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: player,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Control,
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier: baylee_cards_dsl::Modifier::GainControl,
        });
    }
    state.refresh_characteristics();
}
