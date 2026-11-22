//! Sequential APNAP voting (`CR 701.38a–d`, `Effect::Vote`, `Effect::VoteObjects`,
//! `Effect::ExileMostVotedNotControlledByYou`, `Modifier::AdditionalVotes`).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::choice::{CastModeDesc, CastModeKind};
use baylee_cards_dsl::{VoteObjectAction, VoteOption, VoteOutcome, ZoneRef};
use baylee_core::mana::ManaCost;

/// One player's object vote, as it was cast.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ballot {
    /// The voter.
    pub voter: PlayerId,
    /// What they voted for.
    pub choice: ObjectId,
}

/// One player's named-option vote, as it was cast.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OptionBallot {
    /// The voter.
    pub voter: PlayerId,
    /// Zero-based index into `Effect::Vote::options`.
    pub option: u8,
}

/// Queued per-ballot repetitions for [`VoteOutcome::RepeatPerVote`].
#[derive(Clone, Debug, Hash)]
pub(crate) struct VoteQueue {
    pub remaining: Vec<(PlayerId, &'static [Effect])>,
    pub saved_event_player: Option<PlayerId>,
}

/// The ballots already cast and the players still to vote for an object vote.
#[derive(Clone, Debug, Hash)]
pub struct Vote {
    remaining: Vec<PlayerId>,
    asked: Option<PlayerId>,
    mandatory_left: u8,
    optional_left: u8,
    ballots: Vec<Ballot>,
    secret_ballots: Vec<Ballot>,
    filter: &'static baylee_cards_dsl::Filter,
    zone: ZoneRef,
    not_controlled_by_you_only: bool,
    optional: bool,
    secret: bool,
    action: VoteObjectAction,
    if_no_votes: &'static [Effect],
}

impl Vote {
    /// Publicly visible ballots cast so far (`&[]` while a secret vote is in progress).
    #[must_use]
    pub fn ballots(&self) -> &[Ballot] {
        &self.ballots
    }
}

/// Named-option voting state (`Effect::Vote`).
#[derive(Clone, Debug, Hash)]
pub struct OptionVote {
    remaining: Vec<PlayerId>,
    asked: Option<PlayerId>,
    mandatory_left: u8,
    optional_left: u8,
    ballots: Vec<OptionBallot>,
    secret_ballots: Vec<OptionBallot>,
    options: &'static [VoteOption],
    outcome: VoteOutcome,
    secret: bool,
}

impl OptionVote {
    /// Publicly visible option ballots cast so far (`&[]` while a secret vote is in progress).
    #[must_use]
    pub fn ballots(&self) -> &[OptionBallot] {
        &self.ballots
    }
}

fn turn_ordered_voters(state: &GameState, controller: PlayerId) -> Vec<PlayerId> {
    let mut remaining = eval::players(PlayerRel::EachPlayer, state, controller).unwrap_or_default();
    if let Some(start) = remaining.iter().position(|&player| player == controller) {
        remaining.rotate_left(start);
    }
    remaining
}

fn additional_votes_for(state: &GameState, voter: PlayerId) -> (u8, u8) {
    let mut mandatory = 0_u8;
    let mut optional = 0_u8;
    for effect in state.effects.iter() {
        let baylee_cards_dsl::Modifier::AdditionalVotes {
            count,
            optional: is_opt,
        } = effect.modifier
        else {
            continue;
        };
        let applies = match effect.filter {
            crate::effects::EffectFilter::DslControlledBy(_, seats) => seats.contains(voter),
            crate::effects::EffectFilter::DslWithSubtype(filter, _, seats) => seats.map_or(
                *filter == baylee_cards_dsl::Filter::Any || effect.controller == voter,
                |s| s.contains(voter),
            ),
            crate::effects::EffectFilter::Dsl(filter) => {
                if *filter == baylee_cards_dsl::Filter::Any {
                    true
                } else {
                    effect.controller == voter
                }
            }
            crate::effects::EffectFilter::ObjectIs(id, ver) => state
                .object(id)
                .is_some_and(|o| o.version == ver && o.controller == voter),
            crate::effects::EffectFilter::DslAbout(_, _, _) => effect.controller == voter,
        };
        if !applies {
            continue;
        }
        if is_opt {
            optional = optional.saturating_add(count);
        } else {
            mandatory = mandatory.saturating_add(count);
        }
    }
    (mandatory, optional)
}

/// Legacy entry point for [`Effect::ExileMostVotedNotControlledByYou`].
pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
) -> Option<Pending> {
    start_objects_inner(
        state,
        res,
        filter,
        ZoneRef::Battlefield,
        true,
        false,
        false,
        VoteObjectAction::Exile,
        &[],
    )
}

/// Entry point for [`Effect::VoteObjects`].
pub(super) fn start_objects(
    state: &mut GameState,
    res: &mut Resolution,
    op: &Effect,
) -> Option<Pending> {
    let Effect::VoteObjects {
        filter,
        zone,
        optional,
        secret,
        action,
        if_no_votes,
    } = *op
    else {
        unreachable!("Effect::VoteObjects expected");
    };
    start_objects_inner(
        state,
        res,
        filter,
        zone,
        false,
        optional,
        secret,
        action,
        if_no_votes,
    )
}

#[allow(clippy::too_many_arguments)]
fn start_objects_inner(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    zone: ZoneRef,
    not_controlled_by_you_only: bool,
    optional: bool,
    secret: bool,
    action: VoteObjectAction,
    if_no_votes: &'static [Effect],
) -> Option<Pending> {
    advance_objects(
        state,
        res,
        Vote {
            remaining: turn_ordered_voters(state, res.controller),
            asked: None,
            mandatory_left: 0,
            optional_left: 0,
            ballots: Vec::new(),
            secret_ballots: Vec::new(),
            filter,
            zone,
            not_controlled_by_you_only,
            optional,
            secret,
            action,
            if_no_votes,
        },
    )
}

/// Record one object ballot (or optional refusal) and continue.
pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut vote: Vote,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if let Some(voter) = vote.asked {
        if let Some(&choice) = chosen.first() {
            if legal_object_options(state, res, &vote).contains(&choice) {
                if vote.secret {
                    vote.secret_ballots.push(Ballot { voter, choice });
                } else {
                    vote.ballots.push(Ballot { voter, choice });
                    state.journal.record(GameEvent::Voted {
                        player: voter,
                        object: choice,
                    });
                }
            }
        } else {
            // Declining an optional vote ends this voter's turn.
            vote.mandatory_left = 0;
            vote.optional_left = 0;
        }
    }
    advance_objects(state, res, vote)
}

/// Answer `YesNoPrompt::AdditionalVote` for an object vote.
pub(super) fn resume_object_extra_vote(
    state: &mut GameState,
    res: &mut Resolution,
    mut vote: Vote,
    yes: bool,
) -> Option<Pending> {
    if yes {
        vote.mandatory_left = 1;
    } else {
        vote.optional_left = 0;
    }
    advance_objects(state, res, vote)
}

fn advance_objects(state: &mut GameState, res: &mut Resolution, mut vote: Vote) -> Option<Pending> {
    loop {
        if let Some(voter) = vote.asked {
            if state.has_left(voter) {
                vote.asked = None;
                vote.mandatory_left = 0;
                vote.optional_left = 0;
                continue;
            }
            let options = legal_object_options(state, res, &vote);
            if options.is_empty() {
                vote.asked = None;
                vote.mandatory_left = 0;
                vote.optional_left = 0;
                continue;
            }
            if vote.mandatory_left > 0 {
                vote.mandatory_left -= 1;
                let min = u8::from(!vote.optional);
                res.awaiting = Some(AwaitingOp::Vote(vote));
                return Some(Pending::ChooseCards {
                    player: voter,
                    options,
                    min,
                    max: 1,
                    prompt: ChoicePrompt::Vote,
                    total: None,
                });
            }
            if vote.optional_left > 0 {
                vote.optional_left -= 1;
                res.awaiting = Some(AwaitingOp::Vote(vote));
                return Some(Pending::YesNo {
                    player: voter,
                    prompt: YesNoPrompt::AdditionalVote,
                    source: resolving_ability(state, res),
                });
            }
            vote.asked = None;
        }
        let Some(player) = vote.remaining.first().copied() else {
            break;
        };
        vote.remaining.remove(0);
        if state.has_left(player) {
            continue;
        }
        if legal_object_options(state, res, &vote).is_empty() {
            continue;
        }
        let (extra_mand, extra_opt) = additional_votes_for(state, player);
        vote.asked = Some(player);
        vote.mandatory_left = 1_u8.saturating_add(extra_mand);
        vote.optional_left = extra_opt;
    }
    let all = if vote.secret {
        for b in &vote.secret_ballots {
            state.journal.record(GameEvent::Voted {
                player: b.voter,
                object: b.choice,
            });
        }
        &vote.secret_ballots
    } else {
        &vote.ballots
    };
    if all.is_empty() {
        if !vote.if_no_votes.is_empty() {
            let insert = res.pc + 1;
            res.effects
                .splice(insert..insert, flatten(vote.if_no_votes));
        }
        return None;
    }
    apply_object_winners(state, all, vote.zone, vote.action);
    None
}

fn legal_object_options(state: &GameState, res: &Resolution, vote: &Vote) -> Vec<ObjectId> {
    match vote.zone {
        ZoneRef::Battlefield => state
            .battlefield_seen()
            .filter(|&id| {
                state.object(id).is_some_and(|object| {
                    (!vote.not_controlled_by_you_only || object.controller != res.controller)
                        && eval::matches_with_context(
                            vote.filter,
                            state,
                            object,
                            res.controller,
                            res.rule_context(),
                        )
                })
            })
            .collect(),
        ZoneRef::Graveyard => {
            let mut out = Vec::new();
            for seat in 0..state.players.len() {
                let owner = PlayerId::new(u8::try_from(seat).expect("seat fits"));
                for &id in state.zones.list(ZoneLocation::Graveyard(owner)) {
                    if state.object(id).is_some_and(|object| {
                        eval::matches_with_context(
                            vote.filter,
                            state,
                            object,
                            res.controller,
                            res.rule_context(),
                        )
                    }) {
                        out.push(id);
                    }
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

fn apply_object_winners(
    state: &mut GameState,
    ballots: &[Ballot],
    zone: ZoneRef,
    action: VoteObjectAction,
) {
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
    if most == 0 {
        return;
    }
    let winners: Vec<ObjectId> = unique
        .into_iter()
        .filter(|&candidate| votes(candidate) == most)
        .filter(|&winner| {
            state.object(winner).is_some_and(|object| match zone {
                ZoneRef::Battlefield => {
                    object.zone == crate::zone::Zone::Battlefield
                        && !object.status.contains(crate::object::Status::PHASED_OUT)
                }
                ZoneRef::Graveyard => object.zone == crate::zone::Zone::Graveyard,
                _ => false,
            })
        })
        .collect();
    match action {
        VoteObjectAction::Exile => {
            for winner in winners {
                let Some(owner) = state.object(winner).map(|o| o.owner) else {
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
        VoteObjectAction::Destroy { no_regen } => {
            crate::sba::destroy_all(state, &winners, no_regen);
        }
        VoteObjectAction::ReturnToHand => {
            for winner in winners {
                let Some(owner) = state.object(winner).map(|o| o.owner) else {
                    continue;
                };
                let _ = state.move_object(
                    winner,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
    }
}

/// Entry point for [`Effect::Vote`].
pub(super) fn start_options(
    state: &mut GameState,
    res: &mut Resolution,
    options: &'static [VoteOption],
    outcome: VoteOutcome,
    secret: bool,
    op: Effect,
) -> Option<Pending> {
    if res.subject.vote_queue.is_some() {
        step_vote_queue(state, res, op);
        return None;
    }
    if options.is_empty() {
        return None;
    }
    advance_options(
        state,
        res,
        OptionVote {
            remaining: turn_ordered_voters(state, res.controller),
            asked: None,
            mandatory_left: 0,
            optional_left: 0,
            ballots: Vec::new(),
            secret_ballots: Vec::new(),
            options,
            outcome,
            secret,
        },
        op,
    )
}

/// Resume an option vote after `ChooseMode(index)`.
pub(super) fn resume_option_choice(
    state: &mut GameState,
    res: &mut Resolution,
    mut vote: OptionVote,
    index: usize,
) -> Flow {
    let since = state.journal.last_seq();
    let op = res.effects[res.pc];
    if let Some(voter) = vote.asked
        && index < vote.options.len()
    {
        let option = u8::try_from(index).expect("vote options fit u8");
        if vote.secret {
            vote.secret_ballots.push(OptionBallot { voter, option });
        } else {
            vote.ballots.push(OptionBallot { voter, option });
            state.journal.record(GameEvent::VotedOption {
                player: voter,
                source: res.source,
                option,
            });
        }
    }
    if let Some(pending) = advance_options(state, res, vote, op) {
        return next_choice(state, res, since, pending);
    }
    finish_choice(state, res, since)
}

/// Answer `YesNoPrompt::AdditionalVote` for a named-option vote.
pub(super) fn resume_option_extra_vote(
    state: &mut GameState,
    res: &mut Resolution,
    mut vote: OptionVote,
    yes: bool,
) -> Flow {
    let since = state.journal.last_seq();
    let op = res.effects[res.pc];
    if yes {
        vote.mandatory_left = 1;
    } else {
        vote.optional_left = 0;
    }
    if let Some(pending) = advance_options(state, res, vote, op) {
        return next_choice(state, res, since, pending);
    }
    finish_choice(state, res, since)
}

/// Concession handler when the active voter leaves the game mid-option-vote (`CR 800.4a`).
pub(crate) fn skip_departed_option_voter(state: &mut GameState, res: &mut Resolution) -> Flow {
    let since = state.journal.last_seq();
    let Some(AwaitingOp::OptionVote(mut vote)) = res.awaiting.take() else {
        return Flow::Complete;
    };
    let op = res.effects[res.pc];
    vote.asked = None;
    vote.mandatory_left = 0;
    vote.optional_left = 0;
    if let Some(pending) = advance_options(state, res, vote, op) {
        return next_choice(state, res, since, pending);
    }
    finish_choice(state, res, since)
}

fn advance_options(
    state: &mut GameState,
    res: &mut Resolution,
    mut vote: OptionVote,
    op: Effect,
) -> Option<Pending> {
    loop {
        if let Some(voter) = vote.asked {
            if state.has_left(voter) {
                vote.asked = None;
                vote.mandatory_left = 0;
                vote.optional_left = 0;
                continue;
            }
            if vote.mandatory_left > 0 {
                vote.mandatory_left -= 1;
                let options: Vec<CastModeDesc> = (0..vote.options.len())
                    .map(|index| CastModeDesc {
                        index: u32::try_from(index).unwrap_or(u32::MAX),
                        kind: CastModeKind::VoteOption(u8::try_from(index).expect("fits u8")),
                        cost: ManaCost::ZERO,
                    })
                    .collect();
                res.awaiting = Some(AwaitingOp::OptionVote(vote));
                return Some(Pending::ChooseCastMode {
                    player: voter,
                    object: res.source,
                    options,
                });
            }
            if vote.optional_left > 0 {
                vote.optional_left -= 1;
                res.awaiting = Some(AwaitingOp::OptionVote(vote));
                return Some(Pending::YesNo {
                    player: voter,
                    prompt: YesNoPrompt::AdditionalVote,
                    source: resolving_ability(state, res),
                });
            }
            vote.asked = None;
        }
        let Some(player) = vote.remaining.first().copied() else {
            break;
        };
        vote.remaining.remove(0);
        if state.has_left(player) {
            continue;
        }
        let (extra_mand, extra_opt) = additional_votes_for(state, player);
        vote.asked = Some(player);
        vote.mandatory_left = 1_u8.saturating_add(extra_mand);
        vote.optional_left = extra_opt;
    }
    finalize_option_vote(state, res, &vote, op);
    None
}

fn finalize_option_vote(
    state: &mut GameState,
    res: &mut Resolution,
    vote: &OptionVote,
    op: Effect,
) {
    let all = if vote.secret {
        for b in &vote.secret_ballots {
            state.journal.record(GameEvent::VotedOption {
                player: b.voter,
                source: res.source,
                option: b.option,
            });
        }
        &vote.secret_ballots
    } else {
        &vote.ballots
    };
    let counts: Vec<usize> = (0..vote.options.len())
        .map(|idx| all.iter().filter(|b| usize::from(b.option) == idx).count())
        .collect();
    let insert = res.pc + 1;
    match vote.outcome {
        VoteOutcome::MajorityOrTied { tied_option } => {
            let max_votes = counts.iter().copied().max().unwrap_or(0);
            let winners: Vec<usize> = counts
                .iter()
                .enumerate()
                .filter_map(|(i, &c)| (c == max_votes && max_votes > 0).then_some(i))
                .collect();
            let chosen_idx = match winners.as_slice() {
                [unique] => *unique,
                _ => usize::from(tied_option),
            };
            if let Some(opt) = vote.options.get(chosen_idx)
                && !opt.effects.is_empty()
            {
                res.effects.splice(insert..insert, flatten(opt.effects));
            }
        }
        VoteOutcome::AllWinners => {
            let max_votes = counts.iter().copied().max().unwrap_or(0);
            if max_votes > 0 {
                let mut spliced = Vec::new();
                for (i, opt) in vote.options.iter().enumerate() {
                    if counts[i] == max_votes && !opt.effects.is_empty() {
                        spliced.extend(flatten(opt.effects));
                    }
                }
                if !spliced.is_empty() {
                    res.effects.splice(insert..insert, spliced);
                }
            }
        }
        VoteOutcome::PerOptionCount => {
            let mut spliced = Vec::new();
            for (i, opt) in vote.options.iter().enumerate() {
                if counts[i] > 0 && !opt.effects.is_empty() {
                    spliced.extend(flatten(opt.effects));
                }
            }
            if !spliced.is_empty() {
                res.effects.splice(insert..insert, spliced);
            }
        }
        VoteOutcome::RepeatPerVote => {
            let mut items = Vec::new();
            for (i, opt) in vote.options.iter().enumerate() {
                if opt.effects.is_empty() {
                    continue;
                }
                for ballot in all {
                    if usize::from(ballot.option) == i {
                        items.push((ballot.voter, opt.effects));
                    }
                }
            }
            if !items.is_empty() {
                items.reverse();
                res.subject.vote_queue = Some(Box::new(VoteQueue {
                    remaining: items,
                    saved_event_player: res.subject.event_player(),
                }));
                step_vote_queue(state, res, op);
            }
        }
    }
}

fn step_vote_queue(state: &GameState, res: &mut Resolution, op: Effect) {
    loop {
        let next_item = res
            .subject
            .vote_queue
            .as_deref_mut()
            .and_then(|q| q.remaining.pop());
        let Some((voter, effects)) = next_item else {
            if let Some(done) = res.subject.vote_queue.take() {
                res.subject.set_event_player(done.saved_event_player);
            }
            return;
        };
        if state.has_left(voter) {
            continue;
        }
        res.subject.set_event_player(Some(voter));
        let mut program = flatten(effects);
        program.push(op);
        let insert = res.pc + 1;
        res.effects.splice(insert..insert, program);
        return;
    }
}
