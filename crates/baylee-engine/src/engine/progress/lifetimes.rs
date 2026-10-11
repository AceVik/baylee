//! Lifetime cleanup for registered effects and command-zone emblems.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Ends every "for as long as you control [the source]" whose controller no
/// longer controls its source (CR 611.2b). The source leaving is handled as
/// it moves (`GameState::move_object`); this is the other half, a change of
/// control, which moves nothing.
pub(super) fn end_control_durations(state: &mut crate::state::GameState) {
    use baylee_cards_dsl::Duration;
    // "For as long as [the source] remains tapped" (CR 611.2b) ends with
    // the tap, here as the control half does with control.
    let untapped: Vec<ObjectId> = state
        .effects
        .iter()
        .filter(|fx| matches!(fx.duration, Duration::WhileSourceTapped))
        .filter_map(|fx| {
            let source = fx.source?;
            let still = state.object(source).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && o.status.contains(crate::object::Status::TAPPED)
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
            });
            (!still).then_some(source)
        })
        .collect();
    if !untapped.is_empty() {
        state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::WhileSourceTapped)
                && fx.source.is_some_and(|s| untapped.contains(&s))
        });
    }
    let lost: Vec<(ObjectId, PlayerId)> = state
        .effects
        .iter()
        .filter(|fx| matches!(fx.duration, Duration::WhileYouControlSource))
        .filter_map(|fx| {
            let source = fx.source?;
            // A phased-out source is treated as though it does not exist
            // (CR 702.26b): nobody controls it, and a "for as long as" that
            // tracks it ends as it phases out (CR 702.26f).
            let held = state.object(source).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && o.controller == fx.controller
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
            });
            (!held).then_some((source, fx.controller))
        })
        .collect();
    if !lost.is_empty() {
        state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::WhileYouControlSource)
                && fx
                    .source
                    .is_some_and(|s| lost.contains(&(s, fx.controller)))
        });
    }
}

/// Drops every effect for as long as the game lasts that names one object
/// which has since moved: it is a new object the effect never named
/// (CR 400.7), and a control change would otherwise sit in the hashed table
/// for the rest of the game.
/// The static abilities of emblems not yet registered. "Abilities of emblems
/// function in the command zone" (CR 114.4), and an emblem never leaves it,
/// so each is registered once and lasts the game (Wrenn and Realmbreaker's
/// "You may play lands and cast permanent spells from your graveyard").
pub(super) fn emblem_statics(
    state: &crate::state::GameState,
) -> Vec<crate::effects::ContinuousEffect> {
    let mut found = Vec::new();
    for seat in 0..state.players.len() {
        let zone = ZoneLocation::Command(PlayerId::new(seat as u8));
        for &id in state.zones.list(zone) {
            let Some(obj) = state.object(id) else {
                continue;
            };
            if obj.kind != crate::object::ObjectKind::Emblem {
                continue;
            }
            let Some(abilities) = obj.own_abilities.as_ref() else {
                continue;
            };
            for ability in crate::copiable_abilities::AbilityDefs::from(abilities) {
                let AbilityDef::Static(sa) = ability else {
                    continue;
                };
                if state.effects.has_source_ability(id, sa.modifier) {
                    continue;
                }
                found.push(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(id),
                    controller: obj.controller,
                    origin: crate::effects::EffectOrigin::Static,
                    layer: sa.layer,
                    timestamp: obj.timestamp,
                    duration: baylee_cards_dsl::Duration::Indefinitely,
                    filter: crate::effects::EffectFilter::Dsl(&sa.filter),
                    modifier: sa.modifier,
                });
            }
        }
    }
    found
}

pub(super) fn forget_effects_on_moved_objects(state: &mut crate::state::GameState) {
    use baylee_cards_dsl::Duration;
    let stale: Vec<(ObjectId, u32)> = state
        .effects
        .iter()
        .filter(|fx| matches!(fx.duration, Duration::Indefinitely))
        .filter_map(|fx| match fx.filter {
            crate::effects::EffectFilter::ObjectIs(id, version) => state
                .object(id)
                .is_none_or(|o| o.version != version)
                .then_some((id, version)),
            crate::effects::EffectFilter::Dsl(_) => None,
        })
        .collect();
    if !stale.is_empty() {
        state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::Indefinitely)
                && matches!(fx.filter, crate::effects::EffectFilter::ObjectIs(id, version)
                    if stale.contains(&(id, version)))
        });
    }
}
