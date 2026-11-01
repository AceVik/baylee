//! The land drop, when playing a land costs the seat life.
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
use baylee_engine::choice::LegalActions;
use baylee_view::PlayerView;

use crate::activate::printed_list;

/// The margin `activate::life_ok` keeps above a life payment, for the same
/// question asked of a land drop.
const MARGIN: i32 = 5;

/// The land to play out of what is offered, if any is worth its price.
pub(crate) fn pick(view: &PlayerView, legal: &LegalActions) -> Option<ObjectId> {
    let price = price(view);
    if price == 0 {
        return legal.lands.first().copied();
    }
    let life = view.seat(view.seat).map_or(0, |seat| seat.life);
    if life <= price.saturating_add(MARGIN) {
        return None;
    }
    legal
        .lands
        .iter()
        .copied()
        .find(|&land| view.hand.iter().any(|card| card.id == land))
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
