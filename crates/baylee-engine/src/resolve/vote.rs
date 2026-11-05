//! A sequential vote gathered before its outcome is carried out.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// One player's vote, as it was cast.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ballot {
    /// The voter.
    pub voter: PlayerId,
    /// What they voted for.
    pub choice: ObjectId,
}

/// The ballots already cast and the players still to vote, in turn order.
#[derive(Clone, Debug, Hash)]
pub struct Vote {
    remaining: Vec<PlayerId>,
    /// The player asked now, whose answer is the next ballot.
    asked: Option<PlayerId>,
    ballots: Vec<Ballot>,
    filter: &'static baylee_cards_dsl::Filter,
}

impl Vote {
    /// The ballots cast so far, in the order they were cast.
    #[must_use]
    pub fn ballots(&self) -> &[Ballot] {
        &self.ballots
    }
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
            asked: None,
            ballots: Vec::new(),
            filter,
        },
    )
}

/// Record one legal ballot and ask the next player who has a legal choice.
///
/// Each vote is announced as it is cast (CR 608.2d), so the players after it
/// vote knowing it (CR 701.38a): the journal says it now, not when the vote
/// is counted.
pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut vote: Vote,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if let Some(voter) = vote.asked.take()
        && let Some(&choice) = chosen.first()
        && legal_options(state, res, vote.filter).contains(&choice)
    {
        vote.ballots.push(Ballot { voter, choice });
        state.journal.record(GameEvent::Voted {
            player: voter,
            object: choice,
        });
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
        vote.asked = Some(player);
        res.awaiting = Some(AwaitingOp::Vote(vote));
        return Some(Pending::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Vote,
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
fn exile_winners(state: &mut GameState, ballots: &[Ballot]) {
    let votes = |candidate: ObjectId| {
        ballots
            .iter()
            .filter(|ballot| ballot.choice == candidate)
            .count()
    };
    let mut unique = Vec::new();
    for ballot in ballots {
        if !unique.contains(&ballot.choice) {
            unique.push(ballot.choice);
        }
    }
    let most = unique
        .iter()
        .map(|&candidate| votes(candidate))
        .max()
        .unwrap_or(0);
    for winner in unique
        .into_iter()
        .filter(|&candidate| votes(candidate) == most)
    {
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
