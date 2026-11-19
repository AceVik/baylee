//! What a mass effect takes from each side of the table, read off its
//! structure and the view: the reading the `mass` group strikes a sweep by,
//! and the one the layer sizes a sweep's X by ([`super::number`]).

use baylee_cards_dsl::{Amount, Effect, Filter, KeywordSet};
use baylee_core::ids::PlayerId;
use baylee_view::{PlayerView, PublicObject};

use super::is_ally;
use super::rules::read::{has, toughness_left, worth};
use crate::features4::public_object_matches_basic_filter;

/// How a mass effect takes what it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::netplay4) enum Take {
    /// Destroyed, exiled, sacrificed or returned, whatever its size.
    All,
    /// Dealt this much damage: what it kills, unless it is indestructible.
    Damage(i32),
    /// This much less toughness: what that leaves at zero or less, which no
    /// indestructibility saves (CR 704.5f).
    Shrink(i32),
}

impl Take {
    /// Whether `object` is gone after this.
    fn takes(self, object: &PublicObject) -> bool {
        match self {
            Self::All => true,
            Self::Damage(amount) => {
                toughness_left(object) <= amount && !has(object, KeywordSet::INDESTRUCTIBLE)
            }
            Self::Shrink(amount) => toughness_left(object) <= amount,
        }
    }
}

/// What a mass effect sweeps, for the ones that hit both sides alike, and how,
/// with X at `x`.
pub(in crate::netplay4) fn swept(effect: &Effect, x: i32) -> Option<(&'static Filter, Take)> {
    match effect {
        Effect::DestroyAll { filter, .. }
        | Effect::DestroyAllThenForEachDestroyed { filter, .. }
        | Effect::ExileAll { filter, .. }
        | Effect::SacrificeAll { filter }
        | Effect::ReturnAllToHand {
            filter,
            opponents_only: false,
        } => Some((*filter, Take::All)),
        Effect::DealDamageEach { amount, filter } => Some((
            *filter,
            amount_of(*amount, x).map_or(Take::All, Take::Damage),
        )),
        // "All creatures get -X/-X": every creature, whoever controls it.
        Effect::PumpFilter {
            filter,
            controlled_by: None,
            toughness,
            ..
        } if toughness.is_negative() => Some((*filter, Take::Shrink(shrink_of(*toughness, x)?))),
        _ => None,
    }
}

/// An amount this reading can count, with X at `x`.
pub(in crate::netplay4) fn amount_of(amount: Amount, x: i32) -> Option<i32> {
    match amount {
        Amount::Fixed(value) => i32::try_from(value).ok(),
        Amount::X => Some(x),
        _ => None,
    }
}

/// The size of a negative toughness change, with X at `x`.
fn shrink_of(amount: Amount, x: i32) -> Option<i32> {
    match amount {
        Amount::NegX => Some(x),
        Amount::NegXFixed(value) => i32::try_from(value).ok(),
        _ => None,
    }
}

/// Whether what `effect` sweeps depends on X.
pub(in crate::netplay4) fn sweeps_by_x(effect: &Effect) -> bool {
    match (swept(effect, 0), swept(effect, 1)) {
        (Some(low), Some(high)) => low != high,
        _ => false,
    }
}

/// What the swept filter takes from each side, in worth: `(mine, theirs)`.
pub(in crate::netplay4) fn sweep_toll(
    view: &PlayerView,
    me: PlayerId,
    filter: &Filter,
    take: Take,
) -> (i32, i32) {
    let mut toll = (0, 0);
    for object in &view.battlefield {
        if take.takes(object) && public_object_matches_basic_filter(me, object, filter) {
            if is_ally(me, object.controller) {
                toll.0 += worth(object);
            } else {
                toll.1 += worth(object);
            }
        }
    }
    toll
}

/// Whether the sweeps among `effects`, with X at `x`, take `object`.
pub(in crate::netplay4) fn taken(
    me: PlayerId,
    effects: &[Effect],
    x: i32,
    object: &PublicObject,
) -> bool {
    effects
        .iter()
        .filter_map(|effect| swept(effect, x))
        .any(|(filter, take)| {
            take.takes(object) && public_object_matches_basic_filter(me, object, filter)
        })
}

/// The values of X worth trying for the sweeps among `effects`, from `low` to
/// `high`: both ends, and each X that first ends something they name, where
/// X changes what they take. Ascending, each once.
pub(in crate::netplay4) fn x_steps(
    view: &PlayerView,
    me: PlayerId,
    effects: &[Effect],
    low: i32,
    high: i32,
) -> Vec<i32> {
    let mut steps = vec![low, high.max(low)];
    for effect in effects.iter().filter(|effect| sweeps_by_x(effect)) {
        let Some((filter, _)) = swept(effect, 0) else {
            continue;
        };
        steps.extend(
            view.battlefield
                .iter()
                .filter(|object| object.toughness.is_some())
                .filter(|object| public_object_matches_basic_filter(me, object, filter))
                .map(toughness_left)
                .filter(|step| (low..=high).contains(step)),
        );
    }
    steps.sort_unstable();
    steps.dedup();
    steps
}
