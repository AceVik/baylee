//! Linked graveyard exile and its independently targeted reflexive benefit.
use super::{Aim, Origin};
use crate::HeuristicAgent;
use baylee_cards_dsl::{Effect, ReflexiveEvent, TargetReq, TargetSpec, ZoneRef};
use baylee_view::PlayerView;

/// A reflexive target is chosen after exile, independently of the exiled card.
#[allow(clippy::too_many_arguments)] // valuation context plus the two separate target instances
pub(super) fn reflexive_worth(
    agent: &HeuristicAgent,
    view: &PlayerView,
    origin: Origin<'_>,
    preceding: &[Effect],
    when: ReflexiveEvent,
    effects: &[Effect],
    target: Option<TargetSpec>,
    aim: Aim,
    x: u32,
) -> Option<i64> {
    let ReflexiveEvent::Exiled(filter) = when else {
        return None;
    };
    if !preceding
        .iter()
        .any(|effect| matches!(effect, Effect::ExileTargetsWithSource))
    {
        return None;
    }
    // Unread follow-ups stay unread even when this board has no target.
    agent.effects_worth(view, origin, effects, Aim::Nothing, x)?;
    let Aim::Object(id) = aim else {
        return Some(0);
    };
    let Some(card) = view.graveyards.iter().flatten().find(|card| card.id == id) else {
        return Some(0);
    };
    if !agent.filter_matches(filter, view, card, ZoneRef::Graveyard, Some(origin.id))? {
        return Some(0);
    }
    let requirement = target.map(TargetReq::one);
    // An empty recipient list is different from an unread instruction.
    if let Some(requirement) = &requirement
        && agent.aims(view, origin, &requirement.spec)?.is_empty()
    {
        return Some(0);
    }
    agent
        .aimed_worth(view, origin, effects, requirement.as_ref(), x)
        .map(|(worth, _)| worth)
}
