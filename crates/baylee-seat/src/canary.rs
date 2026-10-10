//! A paid canary: one real question to a mind, outside any game
//! (`baylee-seat check --canary`, `docs/llm-seat.md` §"A hosted seat").
//!
//! A model's free check (its entry, the model list, a CLI's login) cannot
//! see a spent credit or an exhausted subscription window; only a call can.
//! The canary asks the cheapest question a seat is ever asked, a colour of
//! two, on an empty board, and takes any legal answer as proof the mind
//! plays. It costs one call, so a seat agent asks it only for a profile
//! whose admin switched it on.

use crate::mind::{DeckList, GameContext, Mind, Request};
use baylee_core::ids::{PlayerId, SeatSet};
use baylee_core::mana::ManaColor;
use baylee_core::preset::FormatId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{CombatView, LogTail, Phase, PlayerView, SeatView, Step};
use std::sync::Arc;
use std::time::Duration;

/// Asks `mind` the canary's question within `budget`.
///
/// # Errors
/// The mind's failure, or an answer that is not one of the two colours, as
/// a sentence.
pub async fn ask(mind: &dyn Mind, budget: Duration) -> Result<(), String> {
    let request = question(budget);
    let answer = tokio::time::timeout(budget, mind.decide(request))
        .await
        .map_err(|_| format!("the canary did not answer within {} s", budget.as_secs()))?
        .map_err(|e| e.to_string())?;
    match answer.action {
        PlayerAction::ChooseColor(color) if [ManaColor::Red, ManaColor::Green].contains(&color) => {
            Ok(())
        }
        other => Err(format!(
            "the canary answered something it was not asked: {other:?}"
        )),
    }
}

/// The question: seat 0 of two, a quiet main phase, pick red or green.
#[must_use]
pub fn question(budget: Duration) -> Request {
    let me = PlayerId::new(0);
    Request {
        context: Arc::new(GameContext {
            game_id: "canary".into(),
            seat: me,
            seats: 2,
            teams: vec![None, None],
            names: vec![String::new(), String::new()],
            format: FormatId::Freeform,
            deck: DeckList::default(),
            decision_secs: None,
        }),
        question: 1,
        view: empty_view(me),
        pending: Pending::ChooseColor {
            player: me,
            options: vec![ManaColor::Red, ManaColor::Green],
        },
        log: LogTail::default(),
        budget,
        retry: None,
        continuing: false,
        held: None,
    }
}

fn empty_view(me: PlayerId) -> PlayerView {
    PlayerView {
        decision_player: Some(me),
        controlled_hands: Vec::new(),
        damage_sources: Vec::new(),
        target_objects: Vec::new(),
        seq: 1,
        seat: me,
        turn: 1,
        phase: Phase::FirstMain,
        step: Step::Main,
        active: me,
        awaiting: Some(me),
        deciding: SeatSet::new(),
        decision_remaining_ms: None,
        clocks: Vec::new(),
        lost: Vec::new(),
        priority_held: false,
        policy_acts: Vec::new(),
        monarch: None,
        day_night: None,
        seats: (0..2)
            .map(|i| SeatView {
                mana_pool: baylee_view::ManaPoolView::default(),
                player: PlayerId::new(i),
                life: 20,
                poison: 0,
                energy: 0,
                hand_count: 0,
                no_max_hand_size: false,
                library_count: 0,
                graveyard_count: 0,
                loss: None,
                house_answered: None,
                commanders: vec![],
                commander_damage: vec![],
            })
            .collect(),
        hand: Vec::new(),
        shared_hands: vec![],
        hand_shared_with: SeatSet::new(),
        hand_requests: SeatSet::new(),
        hand_requested: SeatSet::new(),
        battlefield: Vec::new(),
        stack: Vec::new(),
        graveyards: vec![Vec::new(); 2],
        exile: vec![Vec::new(); 2],
        command: vec![Vec::new(); 2],
        combat: CombatView::default(),
        looking_at: Vec::new(),
        library_tops: Vec::new(),
        owed: None,
        targeting: None,
        casting: None,
        sorcery_lock: None,
        sorceries_have_flash: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScriptedMind;

    /// A mind that answers is a canary that passes; the scripted mind gives
    /// the least answer, the first colour.
    #[tokio::test]
    async fn a_mind_that_answers_passes_the_canary() {
        let mind = ScriptedMind::idle();
        assert_eq!(ask(&mind, Duration::from_secs(5)).await, Ok(()));
    }
}
