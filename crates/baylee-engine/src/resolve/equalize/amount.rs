//! Per-player quantities on the simultaneous selection machinery.

use baylee_cards_dsl::{Amount, Filter, PlayerRel};

use super::{Group, Selection, advance, prompt};
use crate::choice::Pending;
use crate::resolve::{Resolution, amount_for_player, players_of};
use crate::state::GameState;
use crate::zone::ZoneLocation;

pub(in crate::resolve) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    filter: Option<&'static Filter>,
    amount: &Amount,
) -> Option<Pending> {
    let mut players = players_of(who, state, res.controller, res);
    players.retain(|&player| !state.has_left(player));
    let seats = state.players.len();
    let active = usize::from(state.turn.active.get());
    players.sort_by_key(|player| (usize::from(player.get()) + seats - active) % seats);
    // CR 608.2h: every count belongs to the start of this instruction.
    // In particular, earlier players' choices cannot alter later counts.
    let groups = players
        .into_iter()
        .map(|player| {
            let cards = filter.map_or_else(
                || state.zones.list(ZoneLocation::Hand(player)).clone(),
                |filter| {
                    crate::eval::controlled_matching_with_context(
                        state,
                        player,
                        filter,
                        res.controller,
                        res.rule_context(),
                    )
                },
            );
            let remove = usize::try_from(amount_for_player(
                amount,
                state,
                res.controller,
                res,
                player,
            ))
            .unwrap_or(usize::MAX);
            let keep = cards.len().saturating_sub(remove);
            Group {
                player,
                cards,
                keep,
            }
        })
        .filter(|group| group.keep != group.cards.len())
        .collect();
    advance(
        state,
        res,
        Selection {
            hands: filter.is_none(),
            prompt: prompt(filter),
            groups,
            current: 0,
            kept: Vec::new(),
            removals: Vec::new(),
        },
    )
}
