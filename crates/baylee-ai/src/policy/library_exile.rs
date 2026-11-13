//! A private exile choice keeps a card the permission will actually allow.
use super::{DecisionContext, Effect, HeuristicAgent, ObjectId, PlayerView, TypeSet};

pub(super) fn choose(
    agent: &HeuristicAgent,
    view: &PlayerView,
    options: &[ObjectId],
    context: &DecisionContext<'_>,
) -> Option<Vec<ObjectId>> {
    if !agent
        .profile
        .plays(crate::AIProfile::TOKEN_AND_EXILE_CHOICES)
    {
        return None;
    }
    let [Effect::LookAtTopExileFaceDown { cast_types, .. }] = context.effects else {
        return None;
    };
    // Do not reinterpret another question merely because it shares the source.
    if !options
        .iter()
        .all(|id| view.looking_at.iter().any(|o| o.id == *id))
    {
        return None;
    }
    let castable = |id: &ObjectId| {
        view.object(*id)
            .and_then(|object| object.card)
            .and_then(|card| baylee_cards::by_index(card.index))
            .is_some_and(|card| {
                card.faces.iter().any(|face| {
                    face.castable_from_hand
                        && !face.types.contains(TypeSet::LAND)
                        && face.types.intersects(*cast_types)
                })
            })
    };
    let value = HeuristicAgent::card_value(view);
    let card = options.iter().copied().max_by_key(|id| {
        (
            castable(id),
            if agent.profile.card_skill() >= 2 {
                value(id)
            } else {
                0
            },
            std::cmp::Reverse(*id),
        )
    })?;
    Some(vec![card])
}
