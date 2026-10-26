//! Private hand inspections and choices made by someone other than the owner.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::Filter;

/// A random look does not reveal, discard, or rearrange the original hand.
pub(super) fn inspect_random(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    count: Amount,
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let count = amount2(&count, state, res.controller, res) as usize;
    let mut options = state.zones.list(ZoneLocation::Hand(player)).clone();
    state.rng.shuffle(&mut options);
    options.truncate(count);
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::InspectHand);
    Some(Pending::ChooseCards {
        player: res.controller,
        options,
        min: 0,
        max: 0,
        prompt: ChoicePrompt::LookAtHand,
        total: None,
    })
}

/// Preserve the whole inspected hand independently of the selectable subset.
pub(super) fn choose_discard(
    state: &GameState,
    res: &mut Resolution,
    who: PlayerRel,
    count: Amount,
    filter: &'static Filter,
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let inspected = state.zones.list(ZoneLocation::Hand(player)).clone();
    let options: Vec<_> = inspected
        .iter()
        .copied()
        .filter(|id| {
            state.object(*id).is_some_and(|object| {
                eval::matches_with_context(
                    filter,
                    state,
                    object,
                    res.controller,
                    res.rule_context(),
                )
            })
        })
        .collect();
    let count = amount2(&count, state, res.controller, res)
        .min(u32::try_from(options.len()).unwrap_or(u32::MAX));
    Some(offer(res, inspected, player, options, count, Vec::new()))
}

fn offer(
    res: &mut Resolution,
    inspected: Vec<ObjectId>,
    player: PlayerId,
    options: Vec<ObjectId>,
    remaining: u32,
    selected: Vec<ObjectId>,
) -> Pending {
    let count = remaining.min(u32::from(u8::MAX)) as u8;
    res.awaiting = Some(AwaitingOp::PrivateHandDiscard {
        inspected,
        player,
        options: options.clone(),
        remaining,
        selected,
    });
    Pending::ChooseCards {
        player: res.controller,
        options,
        min: count,
        max: count,
        prompt: ChoicePrompt::LookAtHand,
        total: None,
    }
}

#[allow(clippy::too_many_arguments)] // frozen private selection continuation
pub(super) fn discard_chosen(
    state: &mut GameState,
    res: &mut Resolution,
    inspected: Vec<ObjectId>,
    player: PlayerId,
    mut options: Vec<ObjectId>,
    remaining: u32,
    mut selected: Vec<ObjectId>,
    chosen: &[ObjectId],
) -> Option<Pending> {
    selected.extend_from_slice(chosen);
    let remaining = remaining.saturating_sub(chosen.len() as u32);
    if remaining > 0 && !state.has_left(res.controller) {
        options.retain(|card| !chosen.contains(card));
        return Some(offer(res, inspected, player, options, remaining, selected));
    }
    let discards: Vec<_> = selected
        .iter()
        .copied()
        .filter(|card| state.zones.list(ZoneLocation::Hand(player)).contains(card))
        .map(|card| (card, player))
        .collect();
    discard::discard_or_ask(state, res, &discards)
}
