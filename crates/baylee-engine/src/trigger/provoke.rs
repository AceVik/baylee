//! Provoke's synthetic attack trigger (CR 702.39a–b).

use super::events::ObjectMove;
use super::{PendingTrigger, source_at_event_with_moves, times_triggered};
use crate::{
    event::{GameEvent, JournalEntry},
    object::GameObject,
    state::GameState,
    text_changes::TextChangeMap,
};
use baylee_cards_dsl::{Effect, Filter, KeywordSet, TargetReq, TargetSpec};

static DEFENDING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByDefendingPlayer]);

static PROVOKE_INNER: &[Effect] = &[Effect::UntapTarget, Effect::MustBlockSourceIfAbleThisCombat];

static PROVOKE_EFFECTS: &[Effect] = &[Effect::MayDo {
    effects: PROVOKE_INNER,
}];

pub(super) fn collect(
    state: &GameState,
    obj: &GameObject,
    events: &[JournalEntry],
    moves: &[ObjectMove],
    batch_has_attacks: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    if !batch_has_attacks {
        return;
    }
    let permanent = obj.id;
    // CR 702.39a: "Whenever this creature attacks, you may choose to have target
    // creature defending player controls block this creature this combat if
    // able. If you do, untap that creature."
    // CR 702.39b: Each instance of provoke triggers separately.
    for entry in events {
        let GameEvent::BecameAttacker { object, .. } = &entry.event else {
            continue;
        };
        if *object != permanent {
            continue;
        }
        let source = source_at_event_with_moves(state, obj, moves, entry.seq, false);
        if !source
            .characteristics()
            .keywords
            .contains(KeywordSet::PROVOKE)
        {
            continue;
        }
        let instances = crate::layers::keyword_instances(state, source, KeywordSet::PROVOKE);
        for _ in 0..instances.saturating_mul(times_triggered(
            state,
            baylee_cards_dsl::TriggerEventKind::Any,
            permanent,
        )) {
            triggers.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: Some(source.version),
                related_objects: None,
                grantor: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: state.event_object_identity(permanent, entry.seq),
                counter_source_version: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                event_amount: None,
                event_player: None,
                source: permanent,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                controller: source.controller,
                timestamp: source.timestamp,
                event_object: Some(permanent),
                implicit_target: None,
                abilities: None,
                synthetic_effects: Some(PROVOKE_EFFECTS),
                once_per_turn: false,
                synthetic_target: Some(TargetReq::one(TargetSpec::Object(&DEFENDING_CREATURE))),
                chosen_mode: None,
                in_chosen_order: false,
            });
        }
    }
}
