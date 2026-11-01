//! Lifetime cleanup for registered effects and command-zone emblems.
#[allow(clippy::wildcard_imports)]
use super::*;

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

/// Drops every effect for as long as the game lasts that names one object
/// which has since moved: it is a new object the effect never named
/// (CR 400.7), and a control change would otherwise sit in the hashed table
/// for the rest of the game.
pub(super) fn forget_effects_on_moved_objects(state: &mut crate::state::GameState) {
    use baylee_cards_dsl::Duration;
    if !state.effects.may_have_indefinite_object() {
        return;
    }
    let stale: smallvec::SmallVec<[(ObjectId, u32); 4]> = state
        .effects
        .iter()
        .filter(|fx| matches!(fx.duration, Duration::Indefinitely))
        .filter_map(|fx| match fx.filter {
            crate::effects::EffectFilter::ObjectIs(id, version) => state
                .object(id)
                .is_none_or(|o| o.version != version)
                .then_some((id, version)),
            crate::effects::EffectFilter::Dsl(_)
            | crate::effects::EffectFilter::DslAbout(..)
            | crate::effects::EffectFilter::DslControlledBy(..) => None,
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
