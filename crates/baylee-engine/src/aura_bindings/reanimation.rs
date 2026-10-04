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
}

/// Change the enchant ability, then return the card currently enchanted.
/// A failed return still leaves the changed enchant ability in effect.
pub(crate) fn begin_reanimation(
    state: &mut GameState,
    res: &Resolution,
    owner_control: bool,
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
    state
        .move_object(
            host,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .ok()?;
    let returned = state.source_identity(host)?;
    if !live_battlefield(state, returned) {
        return None;
    }
    // Several copied enter triggers may create independent relationships.
    // Keeping all actual returns implements the gained enchant wording;
    // it never grants permission to a later incarnation of one of them.
    state.reanimated_auras.push(ReanimatedAura {
        aura,
        returned: Some(returned),
    });
    Some(ReanimationFinish {
        aura,
        returned,
        controller: res.controller,
        text: res.text,
    })
}

/// Entry is complete: attempt the attachment and define its delayed trigger.
/// Protection can prevent the attachment without preventing the return or
/// the delayed sacrifice when the unattached Aura subsequently leaves.
pub(crate) fn finish_reanimation(state: &mut GameState, finish: ReanimationFinish) {
    if live_battlefield(state, finish.aura)
        && live_battlefield(state, finish.returned)
        && can_attach(state, finish.aura.object, finish.returned.object)
    {
        state.attach(finish.aura.object, finish.returned.object);
    }
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
            effects: &[Effect::SacrificeEvent],
            text: finish.text,
            object: finish.returned.object,
            version: finish.returned.version,
        },
    });
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
    let (owner, controller) = (object.owner, object.controller);
    if state.has_left(controller) {
        return;
    }
    let _ = state.move_object(
        event.object,
        ZoneLocation::Graveyard(owner),
        ZonePosition::Top,
        Cause::Effect,
    );
}
