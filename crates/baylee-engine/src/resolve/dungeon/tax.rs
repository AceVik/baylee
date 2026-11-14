//! Tomb payments: everyone chooses before any payment or life loss happens.
use super::{
    AwaitingOp, Cause, ChoicePrompt, CostPart, DiscardThen, Filter, GameState, ObjectId, Pending,
    PlayerId, Resolution, sacrifice_together,
};

/// Frozen APNAP choices for one room's simultaneous payments.
#[derive(Clone, Debug, Hash)]
pub struct Tax {
    /// Discard rather than sacrifice.
    pub discard: bool,
    /// Oubliette chooses a creature, artifact and land before sacrificing.
    pub oubliette: bool,
    /// Players in APNAP order.
    pub players: Vec<PlayerId>,
    /// Each selected payment, or an explicit decline.
    pub chosen: Vec<Option<ObjectId>>,
}

pub(in crate::resolve) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    discard: bool,
) -> Option<Pending> {
    let start = usize::from(state.turn.active.get());
    let players = (0..state.players.len())
        .map(|offset| state.players[(start + offset) % state.players.len()].id)
        .filter(|p| !state.has_left(*p))
        .collect();
    advance(
        state,
        res,
        Box::new(Tax {
            oubliette: false,
            discard,
            players,
            chosen: Vec::new(),
        }),
    )
}

pub(in crate::resolve) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut tax: Box<Tax>,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if let Some(&player) = tax.players.get(tax.chosen.len()) {
        let picked = chosen.first().copied();
        tax.chosen.push(picked);
        if let Some(card) = picked {
            state.journal.record(crate::event::GameEvent::CardsChosen {
                player,
                cards: vec![card],
                action: if tax.discard {
                    crate::event::ChosenAction::Discard
                } else {
                    crate::event::ChosenAction::Sacrifice
                },
            });
        }
    }
    advance(state, res, tax)
}

impl Tax {
    /// Choices already locked in while later players in APNAP order still have
    /// to answer (CR 101.4b, CR 101.4d).
    #[must_use]
    pub(crate) fn held_choices(&self, state: &GameState) -> Vec<crate::resolve::HeldChoice> {
        let action = if self.discard {
            crate::event::ChosenAction::Discard
        } else {
            crate::event::ChosenAction::Sacrifice
        };
        self.players
            .iter()
            .zip(&self.chosen)
            .filter_map(|(&player, &picked)| {
                let id = picked?;
                // CR 101.4d: a card chosen from a hidden hand stays hidden
                // until the simultaneous discard happens; every seat still
                // sees that `player` chose a card rather than declining.
                let object = if self.discard {
                    None
                } else {
                    state
                        .object(id)
                        .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                        .map(|_| id)
                };
                Some(crate::resolve::HeldChoice {
                    player,
                    object,
                    action,
                })
            })
            .collect()
    }
}

fn advance(state: &mut GameState, res: &mut Resolution, mut tax: Box<Tax>) -> Option<Pending> {
    while let Some(&player) = tax.players.get(tax.chosen.len()) {
        let cost = if tax.oubliette {
            &CostPart::Sacrifice(
                [&Filter::CREATURE, &Filter::ARTIFACT, &Filter::LAND][tax.chosen.len()],
            )
        } else if tax.discard {
            &CostPart::Discard(&Filter::Any)
        } else {
            &CostPart::Sacrifice(&Filter::Or(&[
                Filter::CREATURE,
                Filter::ARTIFACT,
                Filter::LAND,
            ]))
        };
        let mut options = crate::engine::cost_wizard::options(state, player, res.source, cost);
        options.retain(|id| !tax.chosen.contains(&Some(*id)));
        if options.is_empty() || state.has_left(player) {
            tax.chosen.push(None);
            continue;
        }
        let prompt = if tax.discard {
            ChoicePrompt::CostDiscard
        } else {
            ChoicePrompt::CostSacrifice
        };
        let min = u8::from(tax.oubliette);
        res.awaiting = Some(AwaitingOp::DungeonTax(tax));
        return Some(Pending::ChooseCards {
            player,
            options,
            min,
            max: 1,
            prompt,
            total: None,
        });
    }
    let payments: Vec<_> = tax
        .players
        .iter()
        .zip(&tax.chosen)
        .filter_map(|(&player, &card)| card.map(|card| (card, player)))
        .collect();
    if tax.discard {
        if let Some(pending) = super::super::discard::ask_discard_destination(
            state,
            res,
            &payments,
            DiscardThen::Choice {
                op: Box::new(AwaitingOp::DungeonTax(tax.clone())),
                chosen: Vec::new(),
            },
        ) {
            return Some(pending);
        }
        for &(card, player) in &payments {
            state.discard_card(card, player, Cause::Effect, Some(res.controller));
        }
    } else {
        sacrifice_together(
            state,
            &payments.iter().map(|(card, _)| *card).collect::<Vec<_>>(),
        );
    }
    for (&player, card) in tax.players.iter().zip(&tax.chosen) {
        if !tax.oubliette && card.is_none() && !state.has_left(player) {
            state.change_life(player, -2, Cause::Effect);
        }
    }
    None
}

pub(in crate::resolve) fn sacrifice_three(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    advance(
        state,
        res,
        Box::new(Tax {
            discard: false,
            oubliette: true,
            players: vec![res.controller; 3],
            chosen: Vec::new(),
        }),
    )
}
