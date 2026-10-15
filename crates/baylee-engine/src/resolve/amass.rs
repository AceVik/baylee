//! Amass is an non-targeted Army choice, including replacement-created Armies.
use super::{AwaitingOp, ChoicePrompt, GameState, Pending, Resolution, amount2};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use baylee_cards_dsl::{Amount, CounterKind, Duration, Layer, Modifier, TokenDef};
use baylee_core::ids::{EffectId, ObjectId, SubtypeId};

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    token: &'static TokenDef,
    subtype: SubtypeId,
    amount: Amount,
) -> Option<Pending> {
    let Ok(amount) = u16::try_from(amount2(&amount, state, res.controller, res)) else {
        state.numeric_failure = Some("amass counter instruction exceeds u16");
        return None;
    };
    if candidates(state, res.controller).is_empty() {
        let created =
            super::tokens::create_tokens_with_text(state, res.controller, token, None, 1, res.text);
        if created.iter().any(|id| state.entries.contains(*id)) || state.entries.has_after() {
            res.awaiting = Some(AwaitingOp::Entry {
                next: None,
                after: Some(super::after_entry::AfterEntry::Amass { subtype, amount }),
            });
            return None;
        }
    }
    after_entry(state, res, subtype, amount)
}

fn candidates(state: &GameState, controller: baylee_core::ids::PlayerId) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.controller == controller
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                    && o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::ARMY)
            })
        })
        .collect::<Vec<_>>()
}

/// Entry replacements have finished; choose the Army before checking SBAs.
pub(super) fn after_entry(
    state: &mut GameState,
    res: &mut Resolution,
    subtype: SubtypeId,
    amount: u16,
) -> Option<Pending> {
    state.refresh_characteristics();
    let options = candidates(state, res.controller);
    match options.as_slice() {
        [] => None,
        [only] => {
            finish(state, res, *only, subtype, amount);
            None
        }
        _ => {
            res.awaiting = Some(AwaitingOp::Amass { subtype, amount });
            Some(Pending::ChooseCards {
                player: res.controller,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Amass,
                total: None,
            })
        }
    }
}

pub(super) fn finish(
    state: &mut GameState,
    res: &Resolution,
    army: ObjectId,
    subtype: SubtypeId,
    amount: u16,
) {
    crate::replacement::put_counters_by(state, res.controller, army, CounterKind::P1P1, amount);
    let Some(object) = state.object(army) else {
        return;
    };
    if object.characteristics().subtypes.contains(subtype) {
        return;
    }
    let version = object.version;
    // CR 701.47a: a continuous type addition, not a change to copiable values.
    // Non-token creatures (e.g. a changeling) can be Armies too.
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(res.source),
        controller: res.controller,
        origin: EffectOrigin::Resolution,
        layer: Layer::Type,
        timestamp,
        duration: Duration::Indefinitely,
        filter: EffectFilter::ObjectIs(army, version),
        modifier: Modifier::AddSubtype(subtype),
    });
}
