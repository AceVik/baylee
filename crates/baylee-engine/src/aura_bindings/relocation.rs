//! Untargeted Aura relocation within a resolving triggered ability.

use super::can_attach;
use crate::choice::{ChoicePrompt, Pending};
use crate::object::Rider;
use crate::resolve::{AwaitingOp, Resolution};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::Zone;
use baylee_cards_dsl::Filter;
use baylee_core::ids::{DamageSourceRef, ObjectId};

/// The event's exact incarnation, captured before any response could blink it.
pub(super) fn event_reference(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    let object = res.event_object?;
    let version = state
        .object(res.on_stack)?
        .riders
        .iter()
        .find_map(|rider| match rider {
            Rider::EventObjectIdentity(version, _) => Some(*version),
            _ => None,
        })?;
    Some(DamageSourceRef { object, version })
}

pub(super) fn live_battlefield(state: &GameState, reference: DamageSourceRef) -> bool {
    state.object(reference.object).is_some_and(|object| {
        object.version == reference.version
            && object.zone == Zone::Battlefield
            && !object.status.contains(crate::object::Status::PHASED_OUT)
    })
}

/// Destroy the triggering land, then let its controller move the Aura.
/// No state-based action runs between the destruction and the choice.
pub(crate) fn destroy_event_and_offer(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
) -> Option<Pending> {
    let event = event_reference(state, res)?;
    // Read before destruction. A control change before resolution matters;
    // a departed event object supplies its own exact last-known controller.
    let chooser = state.source_object(event)?.controller;
    if live_battlefield(state, event) {
        crate::sba::destroy(state, event.object);
    }
    let aura = DamageSourceRef {
        object: res.source,
        version: crate::resolve::source_version(state, res)?,
    };
    if !live_battlefield(state, aura) || state.has_left(chooser) {
        return None;
    }
    let context = RuleContext {
        source: res.source,
        text: res.text,
    };
    let options: Vec<_> = state
        .battlefield_seen()
        .filter(|&host| {
            can_attach(state, aura.object, host)
                && state.object(host).is_some_and(|object| {
                    crate::eval::matches_with_context(
                        filter,
                        state,
                        object,
                        res.controller,
                        context,
                    )
                })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::AttachAura { aura });
    Some(Pending::ChooseCards {
        player: chooser,
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::NewHost { aura: aura.object },
        total: None,
    })
}

/// The answer is already checked against the offered untargeted choices.
pub(crate) fn resume_attach(state: &mut GameState, aura: DamageSourceRef, chosen: &[ObjectId]) {
    if let Some(&host) = chosen.first()
        && live_battlefield(state, aura)
        && can_attach(state, aura.object, host)
    {
        state.attach(aura.object, host);
    }
}
