//! Death-triggered return keywords, including their counter conditions.
use super::{PendingTrigger, source_at_event, times_triggered};
use crate::{event::GameEvent, object::GameObject, state::GameState, text_changes::TextChangeMap};

/// Undying (CR 702.93a) and persist (CR 702.79a) as the one sentence each
/// of them is: "return it to the battlefield under its owner's control with
/// a +1/+1 (or -1/-1) counter on it".
///
/// `TargetSpec::EventObject` and not a target: neither keyword prints the
/// word, so nothing may be chosen and nothing can be made illegal by a
/// hexproof granted in response — the ability returns the card that just
/// died or it returns nothing.
static UNDYING_RETURN: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::return_to_owner_with(
        baylee_cards_dsl::TargetSpec::EventObject,
        baylee_cards_dsl::CounterKind::P1P1,
        1,
    )];

static PERSIST_RETURN: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::return_to_owner_with(
        baylee_cards_dsl::TargetSpec::EventObject,
        baylee_cards_dsl::CounterKind::M1M1,
        1,
    )];

pub(super) fn collect(
    state: &GameState,
    obj: &GameObject,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    let permanent = obj.id;
    // The **printed** bits and not the projected ones, and that is a
    // limit rather than a choice: `move_object` clears `obj.cache`
    // (CR 400.7), so `characteristics()` has already fallen back to
    // `base` by the time this scan runs, and a continuous effect that
    // *granted* undying stopped applying when the permanent left the
    // battlefield. Closing it needs a fourth look-back store holding
    // the projected keywords, the way `ltb_counters` holds the
    // counters. Mikaeus, the Unhallowed is the one card in this pool
    // that wants it and is `Coverage::Partial` for three reasons of
    // which this is one.
    let keywords = obj.characteristics().keywords;
    for (bit, kind, effects) in [
        (
            baylee_cards_dsl::KeywordSet::UNDYING,
            baylee_cards_dsl::CounterKind::P1P1,
            UNDYING_RETURN,
        ),
        (
            baylee_cards_dsl::KeywordSet::PERSIST,
            baylee_cards_dsl::CounterKind::M1M1,
            PERSIST_RETURN,
        ),
    ] {
        if !keywords.contains(bit) {
            continue;
        }
        // A token has no card to return (CR 111.7): it ceases to
        // exist as a state-based action and the object in `ceased`
        // is all that is left of it. The trigger would resolve onto
        // nothing, which is a rule that looks broken rather than
        // one that declines.
        if obj.card.is_none() {
            continue;
        }
        // The intervening `if` (CR 603.4), and it is read out of
        // the look-back store rather than off the object: the
        // counters were cleared by the very move this trigger is
        // about. Checked **once** and not twice, which is the one
        // place this engine departs from the letter of 603.4 — the
        // rule says on trigger and again on resolution, and both
        // reads are of the same frozen last-known information, so
        // the second cannot answer differently.
        let had = state
            .ltb_counters
            .iter()
            .find(|(id, _)| *id == permanent)
            .map_or(0, |(_, counters)| counters.get(kind));
        if had > 0 {
            continue;
        }
        for entry in events {
            if let GameEvent::ZoneChanged {
                object,
                from,
                to,
                cause: _,
                place: _,
            } = &entry.event
                && *object == permanent
                && *from == crate::zone::Zone::Battlefield
                && *to == crate::zone::Zone::Graveyard
            {
                for _ in
                    0..times_triggered(state, baylee_cards_dsl::TriggerEventKind::Any, permanent)
                {
                    let source = source_at_event(state, obj, events, entry, true);
                    triggers.push(PendingTrigger {
                        text: TextChangeMap::IDENTITY,
                        source_version: Some(source.version),
                        event_object_identity: None,
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
                        implicit_target: Some(permanent),
                        abilities: None,
                        synthetic_effects: Some(effects),
                        once_per_turn: false,
                        synthetic_target: None,
                        chosen_mode: None,
                    });
                }
            }
        }
    }
}
