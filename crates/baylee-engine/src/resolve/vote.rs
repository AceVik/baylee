//! A sequential vote gathered before its outcome is carried out.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// The ballots already cast and the players still to vote, in turn order.
#[derive(Clone, Debug, Hash)]
pub struct Vote {
    remaining: Vec<PlayerId>,
    ballots: Vec<ObjectId>,
    filter: &'static baylee_cards_dsl::Filter,
}

/// Begin with the resolving controller, then proceed in turn order.
pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
) -> Option<Pending> {
    let mut remaining =
        eval::players(PlayerRel::EachPlayer, state, res.controller).unwrap_or_default();
    if let Some(start) = remaining
        .iter()
        .position(|&player| player == res.controller)
    {
        remaining.rotate_left(start);
    }
    advance(
        state,
        res,
        Vote {
            remaining,
            ballots: Vec::new(),
            filter,
        },
    )
}

/// Record one legal ballot and ask the next player who has a legal choice.
pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut vote: Vote,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if let Some(&ballot) = chosen.first()
        && legal_options(state, res, vote.filter).contains(&ballot)
    {
        vote.ballots.push(ballot);
    }
    advance(state, res, vote)
}

/// Skip departed players and players with no legal vote rather than asking an
/// impossible question. The battlefield is small, so keeping the ballots in
/// their deterministic vote order avoids both a hash table and sorted maps.
fn advance(state: &mut GameState, res: &mut Resolution, mut vote: Vote) -> Option<Pending> {
    while let Some(player) = vote.remaining.first().copied() {
        vote.remaining.remove(0);
        if state.has_left(player) {
            continue;
        }
        let options = legal_options(state, res, vote.filter);
        if options.is_empty() {
            continue;
        }
        res.awaiting = Some(AwaitingOp::Vote(vote));
        return Some(Pending::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    exile_winners(state, &vote.ballots);
    None
}

/// Every legal choice is a current, visible permanent the spell's controller
/// does not control. A vote is not a target, so protection does not narrow it.
fn legal_options(
    state: &GameState,
    res: &Resolution,
    filter: &'static baylee_cards_dsl::Filter,
) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|object| {
                object.controller != res.controller
                    && eval::matches_with_context(
                        filter,
                        state,
                        object,
                        res.controller,
                        res.rule_context(),
                    )
            })
        })
        .collect()
}

/// Exile all first places together, including every tie for first place.
fn exile_winners(state: &mut GameState, ballots: &[ObjectId]) {
    let mut unique = Vec::new();
    for &ballot in ballots {
        if !unique.contains(&ballot) {
            unique.push(ballot);
        }
    }
    let most = unique.iter().fold(0_usize, |best, &candidate| {
        best.max(
            ballots
                .iter()
                .filter(|&&ballot| ballot == candidate)
                .count(),
        )
    });
    for winner in unique.into_iter().filter(|&candidate| {
        ballots
            .iter()
            .filter(|&&ballot| ballot == candidate)
            .count()
            == most
    }) {
        let Some(owner) = state
            .object(winner)
            .filter(|object| {
                object.zone == crate::zone::Zone::Battlefield
                    && !object.status.contains(crate::object::Status::PHASED_OUT)
            })
            .map(|object| object.owner)
        else {
            continue;
        };
        let _ = state.move_object(
            winner,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
}
