//! Activation observers read current attachment relationships after costs.

use crate::{eval, object::GameObject, state::GameState, text_changes::RuleContext};
use baylee_cards_dsl::Filter;
use baylee_core::ids::PlayerId;

pub(super) fn source_matches(
    filter: &Filter,
    state: &GameState,
    object: &GameObject,
    you: PlayerId,
    context: RuleContext,
) -> bool {
    let matches = |filter| source_matches(filter, state, object, you, context);
    match filter {
        Filter::And(parts) => parts.iter().all(matches),
        Filter::Or(parts) => parts.iter().any(matches),
        Filter::Not(inner) => !matches(inner),
        // Ordinary filter evaluation includes a departure fallback for death
        // triggers. An activation completes after its costs; it is not a
        // look-back event (CR 602.2b, 603.10). A sacrificed host is no longer
        // enchanted, even while its Aura awaits the next SBA.
        Filter::AttachedToBySource => state.object(context.source).is_some_and(|aura| {
            aura.attached_to == Some(object.id)
                && state.object(object.id).is_some_and(|live| {
                    live.version == object.version && live.zone == crate::zone::Zone::Battlefield
                })
        }),
        _ => eval::matches_with_context(filter, state, object, you, context),
    }
}
