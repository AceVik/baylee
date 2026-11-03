//! Landwalk choices and their value against each defender's actual lands.
use super::Origin;
use crate::HeuristicAgent;
use baylee_cards_dsl::{Duration, Effect, Filter, KeywordSet};
use baylee_core::generated::subtypes::ALL_LAND_TYPES;
use baylee_core::ids::{PlayerId, SubtypeId};
use baylee_core::types::{SubtypeSet, TypeSet};
use baylee_engine::engine::DecisionContext;
use baylee_view::{PlayerView, PublicObject};

/// Defenders with a land of the chosen type, and how many matching lands they have.
struct MatchingLands {
    defenders: Vec<PlayerId>,
    count: usize,
}

/// Read the land filter only where the chosen subtype already matches (CR 702.14c).
fn matching_lands(
    agent: &HeuristicAgent,
    view: &PlayerView,
    origin: Origin<'_>,
    subtype: SubtypeId,
    lands: &Filter,
) -> Option<MatchingLands> {
    let mut defenders = Vec::new();
    let mut count = 0;
    for seat in view
        .seats
        .iter()
        .filter(|seat| !seat.has_lost() && agent.hostile(seat.player, view.seat))
    {
        let mut for_defender = 0;
        for land in view.battlefield_of(seat.player).filter(|object| {
            object.types.contains(TypeSet::LAND) && object.subtypes.contains(subtype)
        }) {
            if agent.filter_matches(
                lands,
                view,
                land,
                baylee_cards_dsl::ZoneRef::Battlefield,
                Some(origin.id),
            )? {
                for_defender += 1;
            }
        }
        if for_defender > 0 {
            defenders.push(seat.player);
            count += for_defender;
        }
    }
    Some(MatchingLands { defenders, count })
}

/// Value the best offered landwalk for these recipients.
///
/// The combat model's `UNBLOCKABLE` bit is applied only against defenders with
/// a matching land, rather than incorrectly making the creature unblockable
/// against the whole table. Other durations and opposing recipients stay
/// unread instead of inventing a permanent or wrong-side value.
pub(super) fn chosen_worth(
    agent: &HeuristicAgent,
    view: &PlayerView,
    origin: Origin<'_>,
    objects: &[&PublicObject],
    lands: &Filter,
    basic_only: bool,
    duration: Duration,
) -> Option<i64> {
    if duration != Duration::UntilEndOfTurn
        || objects.iter().any(|object| object.controller != view.seat)
    {
        return None;
    }
    if objects.is_empty() {
        return Some(0);
    }
    let ids: Vec<_> = objects.iter().map(|object| object.id).collect();
    let mut best = 0;
    for subtype in ALL_LAND_TYPES.iter() {
        if basic_only && !SubtypeSet::BASIC_LANDS.contains(subtype) {
            continue;
        }
        let matching = matching_lands(agent, view, origin, subtype, lands)?;
        if matching.defenders.is_empty() {
            continue;
        }
        let worth = agent.attack_gain_for(
            view,
            &|object| {
                ids.contains(&object.id)
                    .then_some((0, 0, KeywordSet::UNBLOCKABLE.bits()))
            },
            &|player| matching.defenders.contains(&player),
        );
        best = best.max(worth);
    }
    Some(best)
}

/// Answer a subtype question only when its current clause is a landwalk choice.
pub(crate) fn subtype_choice(
    agent: &HeuristicAgent,
    view: &PlayerView,
    options: &[SubtypeId],
    context: &DecisionContext<'_>,
) -> Option<SubtypeId> {
    let lands = match context.effects.first() {
        Some(Effect::ChooseLandwalk { lands, .. }) => *lands,
        Some(Effect::ChooseSourceLandwalk { .. }) => &Filter::Any,
        _ => return None,
    };
    let origin = Origin::of(view, context.source?);
    let mut best: Option<(SubtypeId, usize)> = None;
    for subtype in options.iter().copied() {
        let count = matching_lands(agent, view, origin, subtype, lands)?.count;
        if best.is_none_or(|(_, best_count)| count > best_count) {
            best = Some((subtype, count));
        }
    }
    best.and_then(|(subtype, count)| (count > 0).then_some(subtype))
}
