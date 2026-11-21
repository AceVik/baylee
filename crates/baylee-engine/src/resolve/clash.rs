//! Clash (`CR 701.30a–d`).

use super::{AwaitingOp, Flow, Resolution, resolving_ability};
use crate::choice::{Pending, YesNoPrompt};
use crate::eval;
use crate::event::{Cause, GameEvent};
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Effect, PlayerRel};
use baylee_core::ids::{ObjectId, PlayerId};

/// State of one clashing player during [`Effect::Clash`] (`CR 701.30c–d`).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ClashEntry {
    /// The clashing player.
    pub player: PlayerId,
    /// Revealed top card and its mana value when revealed, or `None` if library was empty.
    pub revealed: Option<(ObjectId, u32)>,
    /// Whether this player chose to put the revealed card on the bottom of their library.
    pub put_bottom: bool,
}

pub(super) fn begin(
    state: &mut GameState,
    res: &mut Resolution,
    on_win: &'static [Effect],
    on_loss: &'static [Effect],
) -> Option<Pending> {
    let opponents =
        eval::players(PlayerRel::EachOpponent, state, res.controller).unwrap_or_default();
    if opponents.len() > 1 {
        res.awaiting = Some(AwaitingOp::ClashOpponent { on_win, on_loss });
        return Some(Pending::ChoosePlayer {
            player: res.controller,
            options: opponents,
        });
    }
    start_with_opponent(state, res, opponents.first().copied(), on_win, on_loss)
}

pub(super) fn resume_opponent(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: PlayerId,
    on_win: &'static [Effect],
    on_loss: &'static [Effect],
) -> Flow {
    let opponent =
        (!state.has_left(chosen) && state.is_opponent(res.controller, chosen)).then_some(chosen);
    if let Some(pending) = start_with_opponent(state, res, opponent, on_win, on_loss) {
        Flow::Wait(pending)
    } else {
        res.pc += 1;
        super::run(state, res)
    }
}

pub(super) fn resume_bottom(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    if !matches!(res.awaiting, Some(AwaitingOp::ClashBottom { .. })) {
        return None;
    }
    let Some(AwaitingOp::ClashBottom {
        mut entries,
        index,
        on_win,
        on_loss,
    }) = res.awaiting.take()
    else {
        unreachable!("matched ClashBottom")
    };
    if let Some(entry) = entries.get_mut(index) {
        entry.put_bottom = answer;
    }
    if let Some(pending) = ask_next_bottom(state, res, entries, index + 1, on_win, on_loss) {
        Some(Flow::Wait(pending))
    } else {
        res.pc += 1;
        Some(super::run(state, res))
    }
}

fn start_with_opponent(
    state: &mut GameState,
    res: &mut Resolution,
    opponent: Option<PlayerId>,
    on_win: &'static [Effect],
    on_loss: &'static [Effect],
) -> Option<Pending> {
    let mut participants = vec![res.controller];
    participants.extend(opponent);
    super::chosen::order_apnap(state, &mut participants);

    let mut entries = Vec::with_capacity(participants.len());
    for player in participants {
        let revealed = state
            .zones
            .list(ZoneLocation::Library(player))
            .last()
            .copied()
            .map(|card| {
                let mv = state
                    .object(card)
                    .map_or(0, |o| o.characteristics().mana_value());
                state.journal.record(GameEvent::Revealed {
                    player,
                    cards: vec![card],
                });
                (card, mv)
            });
        entries.push(ClashEntry {
            player,
            revealed,
            put_bottom: false,
        });
    }
    ask_next_bottom(state, res, entries, 0, on_win, on_loss)
}

fn ask_next_bottom(
    state: &mut GameState,
    res: &mut Resolution,
    entries: Vec<ClashEntry>,
    mut index: usize,
    on_win: &'static [Effect],
    on_loss: &'static [Effect],
) -> Option<Pending> {
    while index < entries.len() {
        let entry = &entries[index];
        if !state.has_left(entry.player)
            && let Some((card, _)) = entry.revealed
        {
            let player = entry.player;
            res.awaiting = Some(AwaitingOp::ClashBottom {
                entries,
                index,
                on_win,
                on_loss,
            });
            return Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::ClashBottom { card },
                source: resolving_ability(state, res),
            });
        }
        index += 1;
    }
    finish_clash(state, res, &entries, on_win, on_loss);
    None
}

fn finish_clash(
    state: &mut GameState,
    res: &mut Resolution,
    entries: &[ClashEntry],
    on_win: &'static [Effect],
    on_loss: &'static [Effect],
) {
    for entry in entries {
        if entry.put_bottom
            && let Some((card, _)) = entry.revealed
            && state.zones.list(ZoneLocation::Library(entry.player)).last() == Some(&card)
        {
            let _ = state.move_object(
                card,
                ZoneLocation::Library(entry.player),
                ZonePosition::Bottom,
                Cause::Effect,
            );
        }
    }
    let mut controller_won = false;
    for (i, entry) in entries.iter().enumerate() {
        let won = entry.revealed.is_some_and(|(_, my_mv)| {
            entries.iter().enumerate().all(|(j, other)| {
                i == j || other.revealed.is_none_or(|(_, other_mv)| my_mv > other_mv)
            })
        });
        if entry.player == res.controller {
            controller_won = won;
        }
        state.journal.record(GameEvent::Clashed {
            player: entry.player,
            won,
        });
    }
    let branch = if controller_won { on_win } else { on_loss };
    if !branch.is_empty() {
        let next = res.pc + 1;
        res.effects.splice(next..next, branch.iter().copied());
    }
}
