//! Flanking's synthetic block trigger.

use super::events::ObjectMove;
use super::{PROWESS_SELF, PendingTrigger, source_at_event_with_moves, times_triggered};
use crate::{
    event::{GameEvent, JournalEntry},
    object::GameObject,
    state::GameState,
    text_changes::TextChangeMap,
};

/// Flanking's "the blocking creature gets -1/-1 until end of turn"
/// (CR 702.25a). The blocker is the trigger's implicit target, so
/// `Filter::This` names it, as it names the permanent for prowess.
static FLANKING_SHRINK: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::CreateContinuousEffect {
        layer: baylee_cards_dsl::Layer::PtModify,
        filter: &PROWESS_SELF,
        modifier: baylee_cards_dsl::Modifier::ModifyPT(-1, -1),
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
    }];

pub(super) fn collect(
    state: &GameState,
    obj: &GameObject,
    events: &[JournalEntry],
    moves: &[ObjectMove],
    batch_has_blocks: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    let permanent = obj.id;
    // Flanking (engine-level keyword trigger, CR 702.25a): "Whenever a
    // creature without flanking blocks this creature, the blocking
    // creature gets -1/-1 until end of turn." Once per blocker, and the
    // blocker's own flanking is read as it blocks. Each instance triggers
    // separately (CR 702.25b).
    if batch_has_blocks
        && obj
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLANKING)
    {
        let instances =
            crate::layers::keyword_instances(state, obj, baylee_cards_dsl::KeywordSet::FLANKING);
        for entry in events {
            if let GameEvent::BecameBlocker { object, attacker } = &entry.event
                && *attacker == permanent
                && state.object(*object).is_some_and(|blocker| {
                    !blocker
                        .characteristics()
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::FLANKING)
                })
            {
                let source = source_at_event_with_moves(state, obj, moves, entry.seq, false);
                for _ in 0..instances.saturating_mul(times_triggered(
                    state,
                    baylee_cards_dsl::TriggerEventKind::Any,
                    permanent,
                )) {
                    triggers.push(PendingTrigger {
                        text: TextChangeMap::IDENTITY,
                        source_version: Some(source.version),
                        related_objects: None,
                        event_subject: None,
                        event_ability: None,
                        event_object_identity: state.event_object_identity(*object, entry.seq),
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
                        event_object: Some(*object),
                        implicit_target: Some(*object),
                        abilities: None,
                        synthetic_effects: Some(FLANKING_SHRINK),
                        once_per_turn: false,
                        synthetic_target: None,
                        chosen_mode: None,
                    });
                }
            }
        }
    }
}
