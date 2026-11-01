//! Two-stage connive and recruit, with draws settled before discard choices.

#[allow(clippy::wildcard_imports)] // the interpreter vocabulary
use super::*;
use baylee_core::ids::DamageSourceRef;
use baylee_core::types::TypeSet;

/// Frozen operands while draws or discard replacements suspend a loot action.
#[derive(Clone, Debug, Hash)]
pub struct Loot {
    /// Controller of the conniving permanent, or the recruiting player.
    pub player: PlayerId,
    /// Number of cards to discard after drawing.
    pub count: u32,
    /// Exact conniving incarnation; absent for recruit.
    pub source: Option<DamageSourceRef>,
    /// Recruit's reward; absent for connive.
    pub token: Option<&'static baylee_cards_dsl::TokenDef>,
    /// Collect large choices in bounded menus, without discarding between menus.
    pub selected: Vec<ObjectId>,
    /// Beginning of this instruction's eligible draw history, if restricted.
    pub drawn_since: Option<usize>,
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    amount: Amount,
    token: Option<&'static baylee_cards_dsl::TokenDef>,
) -> Option<Pending> {
    if let Some(loot) = res.loot.take() {
        return choose_or_finish(state, res, loot);
    }
    let count = amount2(&amount, state, res.controller, res);
    if count == 0 {
        return None;
    }
    let (source, player) = if token.is_some() {
        (None, res.controller)
    } else {
        let reference = subjects::this(state, res)?;
        let player = state.source_object(reference)?.controller;
        (Some(reference), player)
    };
    if state.has_left(player) {
        return None;
    }
    res.loot = Some(Box::new(Loot {
        player,
        count,
        source,
        token,
        selected: Vec::new(),
        drawn_since: None,
    }));
    state.draw_cards(player, count as usize);
    // Keep this instruction until run() has settled all queued draw replacements.
    None
}

/// Draw first, then discard only surviving cards drawn by this instruction.
pub(super) fn drawn_only(
    state: &mut GameState,
    res: &mut Resolution,
    draw: Amount,
    discard: u8,
) -> Option<Pending> {
    if let Some(loot) = res.loot.take() {
        return choose_or_finish(state, res, loot);
    }
    let count = amount2(&draw, state, res.controller, res);
    res.loot = Some(Box::new(Loot {
        player: res.controller,
        count: u32::from(discard),
        source: None,
        token: None,
        selected: Vec::new(),
        drawn_since: Some(state.per_turn.instruction_drawn.len()),
    }));
    state.draw_cards(res.controller, count as usize);
    None
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut loot: Box<Loot>,
    chosen: &[ObjectId],
) -> Option<Pending> {
    loot.selected.extend_from_slice(chosen);
    choose_or_finish(state, res, loot)
}

fn choose_or_finish(
    state: &mut GameState,
    res: &mut Resolution,
    loot: Box<Loot>,
) -> Option<Pending> {
    let selected: std::collections::BTreeSet<_> = loot.selected.iter().copied().collect();
    let options: Vec<_> = state
        .zones
        .list(ZoneLocation::Hand(loot.player))
        .iter()
        .copied()
        .filter(|id| !selected.contains(id))
        .filter(|id| {
            loot.drawn_since.is_none_or(|start| {
                state.per_turn.instruction_drawn[start..]
                    .iter()
                    .any(|(card, version)| {
                        card == id && state.object(*card).is_some_and(|o| o.version == *version)
                    })
            })
        })
        .collect();
    let left = (loot.count as usize)
        .saturating_sub(loot.selected.len())
        .min(options.len());
    if left != 0 && !state.has_left(loot.player) {
        let n = u8::try_from(left).unwrap_or(u8::MAX);
        let player = loot.player;
        res.awaiting = Some(AwaitingOp::Loot(loot));
        return Some(Pending::ChooseCards {
            player,
            options,
            min: n,
            max: n,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    let discards: Vec<_> = loot
        .selected
        .iter()
        .copied()
        .map(|id| (id, loot.player))
        .collect();
    if let Some(pending) = super::discard::ask_discard_destination(
        state,
        res,
        &discards,
        DiscardThen::Choice {
            op: Box::new(AwaitingOp::Loot(loot.clone())),
            chosen: Vec::new(),
        },
    ) {
        return Some(pending);
    }
    let mut nonlands = 0_u32;
    for (card, player) in discards {
        if state.zones.list(ZoneLocation::Hand(player)).contains(&card)
            && state
                .discard_card(card, player, Cause::Effect, Some(res.controller))
                .is_some_and(|types| !types.contains(TypeSet::LAND))
        {
            nonlands = nonlands.saturating_add(1);
        }
    }
    if let Some(source) = loot.source {
        if nonlands > 0 && subjects::on_battlefield(state, source) {
            let Ok(nonlands) = u16::try_from(nonlands) else {
                state.numeric_failure = Some("connive counter amount exceeds the supported range");
                return None;
            };
            crate::replacement::put_counters_by(
                state,
                res.controller,
                source.object,
                baylee_cards_dsl::CounterKind::P1P1,
                nonlands,
            );
        }
        state.journal.record(GameEvent::Connived {
            object: source,
            player: loot.player,
        });
    } else if nonlands > 0
        && let Some(token) = loot.token
    {
        tokens::create_tokens_with_text(state, loot.player, token, None, 1, res.text);
    }
    state.forget_discard_answers();
    None
}

/// Decision progress for the action-loop detector, independent of allocation IDs
/// and the absolute position of this instruction in the turn's draw ledger.
pub(crate) fn loop_progress(state: &GameState, res: &Resolution) -> u64 {
    fn captured(op: &AwaitingOp) -> Option<&Loot> {
        match op {
            AwaitingOp::Loot(loot) => Some(loot),
            AwaitingOp::DiscardDestination {
                then: DiscardThen::Choice { op, .. },
                ..
            }
            | AwaitingOp::SelectedCommander { operation: op, .. } => captured(op),
            _ => None,
        }
    }
    let loot = res
        .awaiting
        .as_ref()
        .and_then(captured)
        .or(res.loot.as_deref());
    let Some(loot) = loot else {
        return 0;
    };
    let hand = state.zones.list(ZoneLocation::Hand(loot.player));
    let eligible: Vec<_> = hand
        .iter()
        .enumerate()
        .filter_map(|(position, &id)| {
            let eligible = loot.drawn_since.is_none_or(|start| {
                state.per_turn.instruction_drawn[start..]
                    .iter()
                    .any(|&(card, version)| {
                        card == id && state.object(card).is_some_and(|o| o.version == version)
                    })
            });
            eligible.then_some(position)
        })
        .collect();
    let mut selected: Vec<_> = loot
        .selected
        .iter()
        .map(|id| hand.iter().position(|card| card == id))
        .collect();
    selected.sort_unstable();
    crate::state::structural_fingerprint(&(loot.player, loot.count, eligible, selected))
}
