//! The land drop: which land is worth playing (a bounce land only with a land
//! to return), which land a bounce land returns, and when playing a land
//! costs the seat life.
//!
//! Fastbond deals its controller 1 damage for each land played after the
//! first in a turn; Ankh of Mishra and Zo-Zu the Punisher deal 2 to whoever
//! a land enters under. Beside a permission to play lands from the graveyard
//! (Crucible of Worlds) and a land that returns lands to the hand (Petrified
//! Field), Fastbond is a loop whose only end is the seat's life, and an agent
//! that plays the first land it is offered walks it to the end: the house,
//! standing in for an absent seat, played land after land out of its
//! graveyard in one turn of a hosted Commander game until Fastbond had dealt
//! it every point it had (feedback of 28.10.2026).
//!
//! The agent cannot count. The view does not say how many lands the seat has
//! played this turn, and the agent answers the same view the same way, so it
//! cannot tell Fastbond's free first land from a later one. It prices every
//! land play at what the triggers would deal if they all fired, and then:
//!
//! - a land that costs nothing is played as before;
//! - a land that costs life is played from the hand only, and only while the
//!   seat keeps the margin `activate::life_ok` keeps for a life payment. A
//!   land in the hand comes back to it a bounded number of times; a land
//!   played from the graveyard is the loop, so it is never played at a price.
//!
//! What that gives up: at six life or less under Fastbond the seat skips even
//! its free first land, and it never ramps through Fastbond and Crucible of
//! Worlds on purpose. Both are cheaper than a seat that kills itself.

use baylee_cards_dsl::{AbilityDef, Amount, Effect, Filter, PlayerRel, TargetSpec, Trigger};
use baylee_core::ids::ObjectId;
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_engine::choice::{ChoicePrompt, LegalActions};
use baylee_view::{PlayerView, StackItem};

use crate::activate::printed_list;

/// The margin `activate::life_ok` keeps above a life payment, for the same
/// question asked of a land drop.
const MARGIN: i32 = 5;

/// The land to play out of what is offered, if any is worth its price.
pub(crate) fn pick(view: &PlayerView, legal: &LegalActions) -> Option<ObjectId> {
    let lands = worth_playing(view, &legal.lands);
    let price = price(view);
    if price == 0 {
        return lands.first().copied();
    }
    let life = view.seat(view.seat).map_or(0, |seat| seat.life);
    if life <= price.saturating_add(MARGIN) {
        return None;
    }
    lands
        .into_iter()
        .find(|&land| view.hand.iter().any(|card| card.id == land))
}

/// The offered lands in the order they should be tried, with the ones that
/// gain nothing left out.
///
/// A bounce land ("Karoo": "when this land enters, return a land you control
/// to its owner's hand") is the one land whose play depends on the rest of the
/// board. Its trigger is not optional and returns a land the seat controls,
/// and the Karoo itself is the only one when the seat has no other: the land
/// drop is spent to end the turn where it began. So it is left out then, and
/// otherwise tried after every other land on offer: a plain land is a whole
/// land now, where the Karoo trades the land it returns for a second mana
/// later. It is played once nothing plainer is offered.
fn worth_playing(view: &PlayerView, offered: &[ObjectId]) -> Vec<ObjectId> {
    let has_land = view
        .battlefield_of(view.seat)
        .any(|o| o.types.contains(TypeSet::LAND));
    let (bounce, plain): (Vec<ObjectId>, Vec<ObjectId>) = offered
        .iter()
        .copied()
        .partition(|&id| returns_a_land(view, id));
    if has_land {
        plain.into_iter().chain(bounce).collect()
    } else {
        plain
    }
}

/// Whether `id`, a card in the seat's hand or a graveyard or a permanent,
/// prints a trigger that returns a land its controller has to their hand.
pub(crate) fn returns_a_land(view: &PlayerView, id: ObjectId) -> bool {
    if let Some(card) = view.hand.iter().find(|card| card.id == id) {
        return baylee_cards::by_index(card.card.index)
            .is_some_and(|def| bounces(def.abilities_for_face(usize::from(card.card.face))));
    }
    view.object(id)
        .is_some_and(|object| bounces(printed_list(object)))
}

/// Whether one of `abilities` is the Karoo trigger.
fn bounces(abilities: &[AbilityDef]) -> bool {
    abilities.iter().any(|ability| {
        matches!(ability, AbilityDef::Triggered { effects, .. }
        if effects.iter().any(|effect| matches!(
            effect,
            Effect::ReturnChosenToHand { who: PlayerRel::You, filter }
                if **filter == Filter::YOUR_LAND
        )))
    })
}

/// Whether the seat's own Karoo trigger is on the stack, so that a card
/// question now asked is the one it puts: which land to return.
fn bounce_pending(view: &PlayerView) -> bool {
    view.stack.iter().any(|entry| {
        entry.controller == view.seat
            && match entry.stack_item {
                Some(StackItem::Ability { source, rules, .. }) => rules.map_or_else(
                    || returns_a_land(view, source),
                    |rules| {
                        baylee_cards::by_index(rules.card).is_some_and(|def| {
                            bounces(def.abilities_for_face(usize::from(rules.face)))
                        })
                    },
                ),
                _ => false,
            }
    })
}

/// The answer to a Karoo trigger's "return a land you control", if `prompt`
/// is that question: one forced card (`min == max == 1`) of a generic prompt,
/// asked while the seat's trigger is on the stack, over lands the seat controls.
/// Answered ahead of the skill gate, as any forced choice is.
pub(crate) fn returned(
    view: &PlayerView,
    options: &[ObjectId],
    min: u8,
    max: u8,
    prompt: ChoicePrompt,
) -> Option<Vec<ObjectId>> {
    let asked = prompt == ChoicePrompt::Generic
        && min == 1
        && max == 1
        && bounce_pending(view)
        && options.iter().all(|id| {
            view.object(*id)
                .is_some_and(|o| o.controller == view.seat && o.types.contains(TypeSet::LAND))
        });
    asked.then(|| to_return(view, options, 1))
}

/// The land to return to the hand, out of the seat's lands on offer.
///
/// Worst for the seat first. A bounce land goes last, because returning it
/// (the Karoo returning itself when it is not alone) leaves the board a land
/// short; then a land that is already tapped before one that is not, since
/// its mana this turn is spent, and a basic land before a nonbasic one, which
/// is replayed without its drawback. Ties go to the lower handle.
pub(crate) fn to_return(view: &PlayerView, options: &[ObjectId], count: usize) -> Vec<ObjectId> {
    let mut ranked = options.to_vec();
    ranked.sort_by_key(|&id| {
        let object = view.object(id);
        (
            returns_a_land(view, id),
            !object.is_some_and(|o| o.status.is_tapped()),
            !object.is_some_and(|o| o.supertypes.contains(SupertypeSet::BASIC)),
            id,
        )
    });
    ranked.truncate(count);
    ranked
}

/// What one land played by the seat deals it, read off the triggers on the
/// battlefield, every one counted as firing.
///
/// Two shapes: the seat's own "whenever you play a land, … deals N damage to
/// you" (Fastbond), and anybody's "whenever a land enters, … deals N damage
/// to that land's controller" (Ankh of Mishra, Zo-Zu the Punisher). A
/// variable amount counts as nothing: no card in the pool prints one here.
pub(crate) fn price(view: &PlayerView) -> i32 {
    view.battlefield
        .iter()
        .flat_map(|object| {
            let own = object.controller == view.seat;
            printed_list(object)
                .iter()
                .map(move |ability| harm(ability, own))
        })
        .fold(0i32, i32::saturating_add)
}

/// What `ability`, on a permanent the seat controls (`own`) or not, deals the
/// seat when it plays a land.
fn harm(ability: &AbilityDef, own: bool) -> i32 {
    let AbilityDef::Triggered {
        trigger, effects, ..
    } = ability
    else {
        return 0;
    };
    let hurts: &dyn Fn(&Effect) -> Option<u32> = match trigger {
        Trigger::PlaysLand(PlayerRel::You) if own => &|effect| match effect {
            Effect::DealDamage {
                amount: Amount::Fixed(n),
                target: TargetSpec::Player(PlayerRel::You),
            }
            | Effect::LoseLife {
                amount: Amount::Fixed(n),
                target: PlayerRel::You,
            } => Some(*n),
            _ => None,
        },
        Trigger::EntersBattlefield(filter) if **filter == Filter::LAND => &|effect| match effect {
            Effect::DealDamage {
                amount: Amount::Fixed(n),
                target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
            } => Some(*n),
            _ => None,
        },
        _ => return 0,
    };
    let total = effects.iter().filter_map(hurts).sum::<u32>();
    i32::try_from(total).unwrap_or(i32::MAX)
}
