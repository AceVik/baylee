//! Balance-style choices: finish every choice before moving any cards.
//!
//! The minimum is sampled separately for each instruction, so land creatures
//! lost in the first instruction no longer count in the creature instruction.
//! Permanent choices are public; hand choices never enter the public journal.

use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

use super::{AwaitingOp, Resolution, chosen};
use crate::choice::{ChoicePrompt, Pending};
use crate::event::{Cause, GameEvent};
use crate::object::ObjectKind;
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};

/// A suspended simultaneous choice. No objects move while this is pending.
#[derive(Clone, Debug)]
pub struct Selection {
    hands: bool,
    prompt: ChoicePrompt,
    groups: Vec<(PlayerId, Vec<ObjectId>)>,
    current: usize,
    keep: usize,
    kept: Vec<ObjectId>,
    removals: Vec<(PlayerId, ObjectId)>,
}

impl Selection {
    /// Choices already made matter even while every card remains in place.
    pub(crate) fn fingerprint(&self) -> u64 {
        let mut key = u64::from(self.hands) + 1;
        for n in [
            self.current,
            self.keep,
            self.kept.len(),
            self.removals.len(),
        ] {
            key = key.wrapping_mul(31).wrapping_add(n as u64);
        }
        for (player, cards) in &self.groups {
            key = key
                .wrapping_mul(31)
                .wrapping_add(u64::from(player.get()) + 1);
            key = key.wrapping_mul(31).wrapping_add(cards.len() as u64);
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
                |filter| chosen::options(state, player, filter, res.controller, res.source),
            );
            (player, options)
        })
        .collect();
    let prompt = match filter {
        None => ChoicePrompt::Keep,
        Some(&Filter::LAND) => ChoicePrompt::KeepLands,
        Some(&Filter::CREATURE) => ChoicePrompt::KeepCreatures,
        Some(_) => ChoicePrompt::KeepPermanents,
    };
    let keep = groups.iter().map(|(_, cards)| cards.len()).min()?;
    advance(
        state,
        res,
        Selection {
            hands: filter.is_none(),
            prompt,
            groups,
            current: 0,
            keep,
            kept: Vec::new(),
            removals: Vec::new(),
        },
    )
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
    while let Some((player, cards)) = selection.groups.get(selection.current) {
        if cards.len() == selection.keep {
            // Keeping everything is forced; no empty or redundant menu.
            selection.kept.clone_from(cards);
        }
        if selection.kept.len() < selection.keep {
            let options = cards
                .iter()
                .copied()
                .filter(|id| !selection.kept.contains(id))
                .collect();
            // Pending's count is a byte, but the number of kept cards is not.
            // Large selections proceed in chunks without truncating the total.
            let count = (selection.keep - selection.kept.len()).min(usize::from(u8::MAX)) as u8;
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
    // No refresh, trigger placement, state-based action or priority between
    // these moves. The resolver refreshes once the entire instruction ends.
    for (player, card) in selection.removals {
        let owner = state.object(card).map_or(player, |object| object.owner);
        if selection.hands {
            state.journal.record(GameEvent::Discarded {
                object: card,
                player,
            });
        } else if let Some(object) = state.object_mut(card) {
            object.kind = ObjectKind::Card;
        }
        let _ = state.move_object(
            card,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    None
}
