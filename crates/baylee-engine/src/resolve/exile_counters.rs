//! Counters placed on cards exiled after death, and counters removed from
//! a card chosen in exile. These are public-zone operations, not targets.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// Public exile piles in seat order, then each pile's stable object order.
/// No hidden zone or hash-map iteration contributes to this choice.
pub(super) fn candidates(
    state: &GameState,
    res: &Resolution,
    owner: PlayerRel,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) -> Vec<ObjectId> {
    if n == 0 {
        return Vec::new();
    }
    let owners = players_of(owner, state, res.controller, res);
    let mut cards = Vec::new();
    for seat in 0..state.players.len() {
        let pile = ZoneLocation::Exile(PlayerId::new(seat as u8));
        cards.extend(state.zones.list(pile).iter().copied().filter(|id| {
            state.object(*id).is_some_and(|object| {
                object.is_card() && owners.contains(&object.owner) && object.counters.get(kind) >= n
            })
        }));
    }
    cards
}

fn exile_event(
    state: &mut GameState,
    res: &Resolution,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) {
    let Some(card) = subjects::event_in_graveyard(state, res) else {
        return;
    };
    let Some(owner) = state.object(card.object).map(|object| object.owner) else {
        return;
    };
    if let Some(object) = state.object_mut(card.object) {
        object.kind = ObjectKind::Card;
    }
    let _ = state.move_object(
        card.object,
        ZoneLocation::Exile(owner),
        ZonePosition::Top,
        Cause::Effect,
    );
    // A replacement may have redirected the move. Counters belong on the
    // exiled incarnation only, never on a card that went somewhere else.
    if state
        .object(card.object)
        .is_some_and(|object| object.zone == crate::zone::Zone::Exile)
    {
        crate::replacement::put_counters_by(state, res.controller, card.object, kind, n);
    }
}

pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    match op {
        Effect::ExileEventFromGraveyardWithCounters { kind, n } => {
            exile_event(state, res, kind, n);
            None
        }
        Effect::RemoveCountersFromChosenExile { owner, kind, n } => {
            let options = candidates(state, res, owner, kind, n);
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::RemoveCountersFromChosenExile { owner, kind, n });
            Some(Pending::ChooseCards {
                player: res.controller,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        _ => unreachable!("only exile-counter operations are dispatched here"),
    }
}

/// Refuse an answer whose public-zone card ceased to be a legal choice.
/// This runs before resuming, so the conditional effects bought by the
/// removal cannot run without their payment.
pub(crate) fn valid_answer(state: &GameState, res: &Resolution, chosen: &[ObjectId]) -> bool {
    let Some(AwaitingOp::RemoveCountersFromChosenExile { owner, kind, n }) = res.awaiting else {
        return true;
    };
    let [card] = chosen else {
        return false;
    };
    candidates(state, res, owner, kind, n).contains(card)
}

pub(super) fn finish_removal(
    state: &mut GameState,
    res: &Resolution,
    owner: PlayerRel,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
    chosen: &[ObjectId],
) {
    let Some(&card) = chosen.first() else {
        return;
    };
    // Keep this check beside the mutation too: direct resolution callers do
    // not necessarily pass through the engine's public-answer validation.
    if candidates(state, res, owner, kind, n).contains(&card) {
        crate::replacement::remove_counters_by(state, card, kind, n, Some(res.controller));
    }
}
