//! Starting chapter choice for Sagas with read ahead (CR 702.155b, CR 715.3b).

use super::{Attempt, GameState, ObjectId, Pending, Question, Zone};
use crate::object::GameObject;
use baylee_cards_dsl::{AbilityDef, KeywordSet, Modifier};

pub(super) fn needs_entry(state: &GameState, id: ObjectId) -> bool {
    let Some(object) = state.object(id) else {
        return false;
    };
    let has_keyword = object
        .characteristics()
        .keywords
        .contains(KeywordSet::READ_AHEAD)
        || state.effects.iter().any(|effect| match effect.modifier {
            Modifier::AddKeyword(kw) => kw.contains(KeywordSet::READ_AHEAD),
            Modifier::BecomeCopyOf(_) => true,
            _ => false,
        });
    has_keyword
        && (object
            .abilities(state.bases.as_ref())
            .iter()
            .any(AbilityDef::is_saga_chapter)
            || state
                .effects
                .iter()
                .any(|effect| matches!(effect.modifier, Modifier::BecomeCopyOf(_))))
}

fn prospective(state: &GameState, id: ObjectId) -> Option<GameObject> {
    let mut object = state.object(id)?.clone();
    object.zone = Zone::Battlefield;
    object.version = object.version.wrapping_add(1);
    let layers =
        crate::layers::LayerPlan::before_entries(&state.effects, super::unavailable(state));
    let projection = crate::layers::recompute_with(state, &object, &layers);
    object.cache.store(projection.characteristics, &object.base);
    object.controller = projection.controller;
    Some(object)
}

pub(super) fn question(state: &GameState, entry: &mut Attempt) -> Option<(Question, Pending)> {
    if entry.refusal.is_some() {
        return None;
    }
    if !needs_entry(state, entry.object) {
        entry.saga_chapter = None;
        return None;
    }
    let object = prospective(state, entry.object)?;
    entry.controller = object.controller;
    if !object
        .characteristics()
        .keywords
        .contains(KeywordSet::READ_AHEAD)
    {
        entry.saga_chapter = None;
        return None;
    }
    let max_chapter = object
        .abilities(state.bases.as_ref())
        .iter()
        .map(AbilityDef::max_saga_chapter)
        .max()
        .unwrap_or(0);
    if max_chapter == 0 {
        entry.saga_chapter = None;
        return None;
    }
    if entry
        .saga_chapter
        .is_some_and(|ch| ch >= 1 && ch <= max_chapter)
    {
        return None;
    }
    if max_chapter == 1 {
        entry.saga_chapter = Some(1);
        return None;
    }
    entry.saga_chapter = None;
    Some((
        Question::ReadAhead,
        Pending::ChooseNumber {
            player: entry.controller,
            min: 1,
            max: u32::from(max_chapter),
            reason: crate::choice::NumberPrompt::ReadAhead { saga: entry.object },
        },
    ))
}
