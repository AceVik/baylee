//! Returning an Aura's graveyard host and retaining the independent leave trigger.

use super::relocation::{event_reference, live_battlefield};
use super::{ReanimatedAura, can_attach};
use crate::event::Cause;
use crate::object::ObjectKind;
use crate::resolve::Resolution;
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState};
use crate::text_changes::TextChangeMap;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::Effect;
use baylee_core::ids::{DamageSourceRef, PlayerId};

/// The second half runs after all entry replacements and static registration,
/// before any state-based action or triggered ability is put on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ReanimationFinish {
    pub(crate) aura: DamageSourceRef,
    pub(crate) returned: DamageSourceRef,
    pub(crate) controller: PlayerId,
    pub(crate) text: TextChangeMap,
    pub(crate) attach: bool,
    pub(crate) leave_action: Option<baylee_cards_dsl::SourceLeaveAction>,
}

/// Change the enchant ability, then return the card currently enchanted.
/// A failed return still leaves the changed enchant ability in effect.
pub(crate) fn begin_reanimation(
    state: &mut GameState,
    res: &Resolution,
    owner_control: bool,
    tapped: bool,
) -> Option<ReanimationFinish> {
    let aura = DamageSourceRef {
        object: res.source,
        version: crate::resolve::source_version(state, res)?,
    };
    if !live_battlefield(state, aura) {
        return None;
    }
    let host = state.object(aura.object)?.attached_to;
    if !state
        .reanimated_auras
        .iter()
        .any(|binding| binding.aura == aura)
    {
        state.reanimated_auras.push(ReanimatedAura {
            aura,
            returned: None,
        });
    }
    let host = host?;
    let object = state.object(host)?;
    if object.zone != Zone::Graveyard || object.card.is_none() {
        return None;
    }
    let controller = if owner_control {
        object.owner
    } else {
        res.controller
    };
    if state.has_left(controller) {
        return None;
    }
    if let Some(object) = state.object_mut(host) {
        object.kind = ObjectKind::Permanent;
        object.set_controller(controller);
    }
    if tapped {
        state.set_tapped(host, true);
    }
    state
        .move_object(
            host,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .ok()?;
    let mut returned = state.source_identity(host)?;
    if state.entries.contains(host) {
        returned.version = returned.version.wrapping_add(1);
    } else if !live_battlefield(state, returned) {
        return None;
    }
    Some(ReanimationFinish {
        aura,
        returned,
        controller: res.controller,
        text: res.text,
        attach: true,
        leave_action: Some(baylee_cards_dsl::SourceLeaveAction::Sacrifice),
    })
}

const SACRIFICE_EVENT_EFFECTS: &[Effect] = &[Effect::SacrificeEvent];
const EXILE_EVENT_EFFECTS: &[Effect] = &[Effect::Exile {
    target: baylee_cards_dsl::TargetSpec::EventObject,
}];

/// Entry is complete: attempt the attachment and define its delayed trigger.
/// Protection can prevent the attachment without preventing the return or
/// the delayed sacrifice when the unattached Aura subsequently leaves.
pub(crate) fn finish_reanimation(state: &mut GameState, finish: ReanimationFinish) {
    if !live_battlefield(state, finish.returned) {
        return;
    }
    if finish.attach {
        if state.object(finish.aura.object).is_some_and(|o| {
            o.characteristics()
                .subtypes
                .contains(baylee_core::generated::subtypes::enchantment::AURA)
        }) {
            state.reanimated_auras.push(ReanimatedAura {
                aura: finish.aura,
                returned: Some(finish.returned),
            });
        }
        if live_battlefield(state, finish.aura)
            && live_battlefield(state, finish.returned)
            && can_attach(state, finish.aura.object, finish.returned.object)
        {
            state.attach(finish.aura.object, finish.returned.object);
        }
    }
    if let Some(leave_action) = finish.leave_action {
        let effects = match leave_action {
            baylee_cards_dsl::SourceLeaveAction::Sacrifice => SACRIFICE_EVENT_EFFECTS,
            baylee_cards_dsl::SourceLeaveAction::Exile => EXILE_EVENT_EFFECTS,
        };
        state.delayed.push(DelayedTrigger {
            controller: finish.controller,
            when: DelayedWhen::LeavesBattlefield {
                card: finish.aura.object,
                version: finish.aura.version,
                after: state.journal.last_seq(),
            },
            action: DelayedAction::TriggerAbout {
                source: finish.aura.object,
                source_version: finish.aura.version,
                effects,
                text: finish.text,
                object: finish.returned.object,
                version: finish.returned.version,
                subject: None,
            },
        });
    }
}

/// "That creature's controller sacrifices it" follows the object even if
/// control or card types changed, but never follows it through a zone change.
pub(crate) fn sacrifice_event(state: &mut GameState, res: &Resolution) {
    let Some(event) = event_reference(state, res) else {
        return;
    };
    if !live_battlefield(state, event) {
        return;
    }
    let Some(object) = state.object(event.object) else {
        return;
    };
    let controller = object.controller;
    if state.has_left(controller) || !state.can_be_sacrificed(event.object, Some(res.controller)) {
        return;
    }
    let _ = state.sacrifice_object(event.object, Cause::Effect);
}
