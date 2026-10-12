//! Untargeted Aura attachment choices before entering (CR 303.4f–g).
use super::{Attempt, ChoicePrompt, GameState, ObjectId, Pending, Question, Zone, ZoneLocation};

pub(super) fn needs_choice(state: &GameState, id: ObjectId) -> bool {
    state.object(id).is_some_and(|object| {
        !matches!(object.zone, Zone::Stack | Zone::Battlefield)
            && object.attached_to.is_none()
            && object
                .characteristics()
                .subtypes
                .contains(baylee_core::generated::subtypes::enchantment::AURA)
    })
}

pub(super) fn question(state: &GameState, entry: &mut Attempt) -> Option<(Question, Pending)> {
    if entry.refusal.is_some() || !needs_choice(state, entry.object) {
        return None;
    }
    let options: Vec<_> = state
        .battlefield_seen()
        .chain(state.players.iter().flat_map(|player| {
            state
                .zones
                .list(ZoneLocation::Graveyard(player.id))
                .iter()
                .copied()
        }))
        .filter(|host| crate::aura_bindings::can_attach(state, entry.object, *host))
        .collect();
    if options.is_empty() {
        entry.refusal = Some(super::Refusal::StayInZone);
        return None;
    }
    Some((
        Question::AuraHost,
        Pending::ChooseCards {
            player: entry.controller,
            options,
            min: 1,
            max: 1,
            prompt: ChoicePrompt::NewHost { aura: entry.object },
            total: None,
        },
    ))
}
