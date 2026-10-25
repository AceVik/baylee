//! Balance-style choices: finish every choice before moving any cards.
//!
//! The minimum is sampled separately for each instruction, so land creatures
//! lost in the first instruction no longer count in the creature instruction.
//! Permanent choices are public; hand choices never enter the public journal.

use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

use super::{AwaitingOp, Resolution};
use crate::choice::{ChoicePrompt, Pending};
use crate::event::GameEvent;
use crate::state::GameState;
use crate::zone::ZoneLocation;

mod amount;
pub(super) use amount::start as start_amount;

#[derive(Clone, Debug)]
struct Group {
    player: PlayerId,
    cards: Vec<ObjectId>,
    keep: usize,
}

/// A suspended simultaneous choice. No objects move while this is pending.
#[derive(Clone, Debug)]
pub struct Selection {
    hands: bool,
    prompt: ChoicePrompt,
    groups: Vec<Group>,
    current: usize,
    kept: Vec<ObjectId>,
    removals: Vec<(PlayerId, ObjectId)>,
}

impl Selection {
    /// Choices already made matter even while every card remains in place.
    pub(crate) fn fingerprint(&self) -> u64 {
        let mut key = u64::from(self.hands) + 1;
        for n in [self.current, self.kept.len(), self.removals.len()] {
            key = key.wrapping_mul(31).wrapping_add(n as u64);
        }
        for Group {
            player,
            cards,
            keep,
        } in &self.groups
        {
            key = key
                .wrapping_mul(31)
                .wrapping_add(u64::from(player.get()) + 1);
            key = key.wrapping_mul(31).wrapping_add(cards.len() as u64);
            key = key.wrapping_mul(31).wrapping_add(*keep as u64);
            for id in cards {
                key = mix_object(key, *id);
            }
        }
        for id in &self.kept {
            key = mix_object(key, *id);
        }
        for (player, id) in &self.removals {
            key = key
                .wrapping_mul(31)
                .wrapping_add(u64::from(player.get()) + 1);
            key = mix_object(key, *id);
        }
        key
    }
}

fn mix_object(key: u64, id: ObjectId) -> u64 {
    key.wrapping_mul(31)
        .wrapping_add(u64::from(id.generation()))
        .wrapping_mul(31)
        .wrapping_add(u64::from(id.slot()) + 1)
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    filter: Option<&'static Filter>,
) -> Option<Pending> {
    let seats = state.players.len();
    let active = usize::from(state.turn.active.get());
    let groups: Vec<_> = (0..seats)
        .map(|offset| PlayerId::new(((active + offset) % seats) as u8))
        .filter(|&player| !state.has_left(player))
        .map(|player| {
            let options = filter.map_or_else(
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
            (player, options)
        })
        .collect();
    let keep = groups.iter().map(|(_, cards)| cards.len()).min()?;
    let groups = groups
        .into_iter()
        .map(|(player, cards)| Group {
            player,
            cards,
            keep,
        })
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

fn prompt(filter: Option<&'static Filter>) -> ChoicePrompt {
    match filter {
        None => ChoicePrompt::Keep,
        Some(&Filter::LAND) => ChoicePrompt::KeepLands,
        Some(&Filter::CREATURE) => ChoicePrompt::KeepCreatures,
        Some(_) => ChoicePrompt::KeepPermanents,
    }
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Option<Pending> {
    selection.kept.extend_from_slice(chosen);
    advance(state, res, selection)
}

fn advance(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
) -> Option<Pending> {
    while let Some(Group {
        player,
        cards,
        keep,
    }) = selection.groups.get(selection.current)
    {
        if cards.len() == *keep {
            // Keeping everything is forced; no empty or redundant menu.
            selection.kept.clone_from(cards);
        }
        if selection.kept.len() < *keep {
            let options = cards
                .iter()
                .copied()
                .filter(|id| !selection.kept.contains(id))
                .collect();
            // Pending's count is a byte, but the number of kept cards is not.
            // Large selections proceed in chunks without truncating the total.
            let count = (*keep - selection.kept.len()).min(usize::from(u8::MAX)) as u8;
            let pending = Pending::ChooseCards {
                player: *player,
                options,
                min: count,
                max: count,
                prompt: selection.prompt,
                total: None,
            };
            res.awaiting = Some(AwaitingOp::Equalize(Box::new(selection)));
            return Some(pending);
        }
        if !selection.hands && !selection.kept.is_empty() {
            state.journal.record(GameEvent::CardsKept {
                player: *player,
                cards: selection.kept.clone(),
            });
        }
        selection.removals.extend(
            cards
                .iter()
                .copied()
                .filter(|id| !selection.kept.contains(id))
                .map(|id| (*player, id)),
        );
        selection.kept.clear();
        selection.current += 1;
    }
    // Hands are discarded (Balance): Library of Leng is asked first, and its
    // answer discards exactly these cards.
    if selection.hands {
        let discards: Vec<(ObjectId, PlayerId)> = selection
            .removals
            .iter()
            .map(|&(player, card)| (card, player))
            .collect();
        return super::discard_or_ask(state, res, &discards);
    }
    // No refresh, trigger placement, state-based action or priority between
    // these moves. The resolver refreshes once the entire instruction ends.
    let victims: Vec<_> = selection
        .removals
        .into_iter()
        .map(|(_, card)| card)
        .collect();
    super::sacrifice_together(state, &victims);
    None
}
