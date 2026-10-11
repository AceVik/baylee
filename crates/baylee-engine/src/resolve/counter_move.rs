//! Move exactly one counter, without removing it if the destination cannot take it.
use super::{AwaitingOp, Flow, GameState, Pending, Resolution, run, subjects};
use crate::choice::{CastModeDesc, CastModeKind};
use baylee_cards_dsl::{CounterKind, TargetSlot};
use baylee_core::{ids::DamageSourceRef, mana::ManaCost};

fn subject(state: &GameState, res: &Resolution, slot: TargetSlot) -> Option<DamageSourceRef> {
    match slot {
        TargetSlot::This => subjects::source(state, res),
        TargetSlot::First => subjects::target(state, res, *res.targets.first()?),
        TargetSlot::Second => subjects::target(state, res, *res.second_targets.first()?),
    }
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    from: TargetSlot,
    to: TargetSlot,
    kind: Option<CounterKind>,
) -> Option<Pending> {
    let from = subject(state, res, from)?;
    let to = subject(state, res, to)?;
    if let Some(kind) = kind {
        crate::replacement::move_counter(state, res.controller, from, to, kind);
        return None;
    }
    let kinds: Vec<_> = state
        .object(from.object)?
        .counters
        .iter()
        .map(|(kind, _)| kind)
        .filter(|kind| crate::replacement::can_move_counter(state, from, to, *kind))
        .collect();
    match kinds.as_slice() {
        [] => None,
        [kind] => {
            crate::replacement::move_counter(state, res.controller, from, to, *kind);
            None
        }
        _ => {
            let options = kinds
                .iter()
                .enumerate()
                .map(|(index, kind)| CastModeDesc {
                    index: u32::try_from(index).expect("finite counter-kind vocabulary"),
                    kind: CastModeKind::Counter(*kind),
                    cost: ManaCost::ZERO,
                })
                .collect();
            res.awaiting = Some(AwaitingOp::MoveCounter { from, to, kinds });
            Some(Pending::ChooseCastMode {
                player: res.controller,
                object: res.source,
                options,
            })
        }
    }
}

pub(super) fn resume(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    let Some(AwaitingOp::MoveCounter { from, to, kinds }) = res.awaiting.take() else {
        unreachable!("counter-kind choice")
    };
    let kind = *kinds.get(index).expect("validated counter-kind choice");
    crate::replacement::move_counter(state, res.controller, from, to, kind);
    res.pc += 1;
    run(state, res)
}
