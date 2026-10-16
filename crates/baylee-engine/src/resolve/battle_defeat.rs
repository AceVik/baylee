//! Resolve the intrinsic Siege trigger against only its original incarnation.
use super::{AwaitingOp, ChoicePrompt, GameState, Pending, Resolution, source_version};
use crate::event::Cause;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::ids::DamageSourceRef;

pub(super) fn start(state: &mut GameState, res: &mut Resolution, cast: bool) -> Option<Pending> {
    let version = source_version(state, res)?;
    let source = state
        .object(res.source)
        .filter(|object| object.version == version && object.zone == Zone::Battlefield)?;
    let owner = source.owner;
    let moved = state
        .move_object(
            res.source,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .ok()?;
    if !cast {
        return None;
    }
    let object = state
        .object(moved)
        .filter(|o| o.zone == Zone::Exile && o.is_card())?;
    let card = DamageSourceRef {
        object: moved,
        version: object.version,
    };
    res.awaiting = Some(AwaitingOp::BattleDefeatCast { card });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: vec![moved],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::PlayFromExile,
        total: None,
    })
}
