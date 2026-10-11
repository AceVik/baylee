//! Tap, pay energy, then choose counter recipients without imposing a page-size rules limit.
use super::{AwaitingOp, ChoicePrompt, GameState, Pending, Resolution, eval, players_of};
use baylee_cards_dsl::{CounterKind, Filter, PlayerRel};
use baylee_core::ids::{DamageSourceRef, ObjectId};

/// Exact incarnations that became tapped, preserved through both choices.
#[derive(Clone, Debug)]
pub struct Selection {
    objects: Vec<DamageSourceRef>,
    next: usize,
    chosen: Vec<DamageSourceRef>,
    remaining: u32,
    kind: CounterKind,
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    filter: &Filter,
    kind: CounterKind,
) -> Option<Pending> {
    let seats = players_of(who, state, res.controller, res);
    let matching: Vec<_> = state
        .battlefield_seen()
        .filter_map(|id| {
            let o = state.object(id)?;
            (seats.contains(&o.controller)
                && eval::matches_with_context(filter, state, o, res.controller, res.rule_context()))
            .then_some(DamageSourceRef {
                object: id,
                version: o.version,
            })
        })
        .collect();
    let objects: Vec<_> = matching
        .into_iter()
        .filter(|r| super::tap(state, r.object))
        .collect();
    let max = u32::from(state.players[usize::from(res.controller.get())].energy);
    if max == 0 {
        return None;
    }
    let candidates = u32::try_from(objects.len()).expect("object IDs bound the battlefield");
    res.awaiting = Some(AwaitingOp::TapEnergy(Box::new(Selection {
        objects,
        next: 0,
        chosen: Vec::new(),
        remaining: 0,
        kind,
    })));
    Some(Pending::ChooseNumber {
        player: res.controller,
        min: 0,
        max,
        reason: crate::choice::NumberPrompt::EnergyPayment { candidates },
    })
}

pub(super) fn pay(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    number: u32,
) -> Option<Pending> {
    if !state.pay_energy(res.controller, number) {
        return None;
    }
    selection.remaining = number;
    advance(state, res, selection)
}

pub(super) fn objects(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let end = (selection.next + usize::from(u8::MAX)).min(selection.objects.len());
    selection.chosen.extend(
        selection.objects[selection.next..end]
            .iter()
            .filter(|r| chosen.contains(&r.object))
            .copied(),
    );
    selection.remaining -= u32::try_from(chosen.len()).expect("bounded choice page");
    selection.next = end;
    advance(state, res, selection)
}

fn advance(state: &mut GameState, res: &mut Resolution, selection: Selection) -> Option<Pending> {
    if selection.remaining > 0 && selection.next < selection.objects.len() {
        let end = (selection.next + usize::from(u8::MAX)).min(selection.objects.len());
        let options: Vec<_> = selection.objects[selection.next..end]
            .iter()
            .map(|r| r.object)
            .collect();
        let max = u8::try_from(options.len())
            .expect("bounded choice page")
            .min(u8::try_from(selection.remaining).unwrap_or(u8::MAX));
        let prompt = ChoicePrompt::TappedForCounters {
            kind: selection.kind,
        };
        res.awaiting = Some(AwaitingOp::TapEnergy(Box::new(selection)));
        return Some(Pending::ChooseCards {
            player: res.controller,
            options,
            min: 0,
            max,
            prompt,
            total: None,
        });
    }
    // All recipients are chosen before any counters are placed. A zone round
    // trip cannot make a new incarnation eligible (CR 400.7).
    for reference in selection.chosen {
        if super::subjects::on_battlefield(state, reference) {
            crate::replacement::put_counters_by(
                state,
                res.controller,
                reference.object,
                selection.kind,
                1,
            );
        }
    }
    None
}
