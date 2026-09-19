//! Ward across the real engine boundary, and the half of it the view cannot
//! carry yet.
//!
//! The unit test beside `policy::pays_tax` builds ward's question; this one
//! earns it. Ward is synthesised as a trigger on the **caster**
//! (`trigger.rs`, CR 702.21) and only arrives once the spell is on the stack
//! and its target is chosen, so nothing short of a game proves that the
//! agent casts into it, is asked, reads the alternative and answers.
//!
//! It answers *yes* — where the old arm declined every tax beside a kicker —
//! and the spell is **still** countered, which is the second assertion and
//! the point of this file. Saying yes with an empty pool is handed a mana
//! window (CR 605.3a, `actions.rs`), and in that window the agent holds
//! priority over two untapped Plains with nothing castable and no reason to
//! tap them: `PlayerView` carries no outstanding payment. It cannot, today —
//! there is no field for it and nothing projects the engine's `mana_window`.
//!
//! So the limitation is pinned rather than described. The day the view says
//! what a seat owes, the second assertion fails, and what replaces it is the
//! game this file was standing in for: Swords to Plowshares resolving and
//! the Throne gone.

use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_engine::engine::Engine;
use baylee_gamehost::{RegistryLookup, SeatContext, player_view};

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

/// A two-seat table with **both** battlefields dealt, which is what separates
/// this from `ai_decisions`'s `position`: the card being targeted belongs to
/// the other seat.
fn table(hand: &[&str], mine: &[&str], theirs: &[&str]) -> GamePreset {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Plains").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = mine.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    preset
}

#[test]
fn the_agent_pays_ward_and_then_cannot_find_the_window_it_was_handed() {
    // Three Plains: one casts Swords to Plowshares, two answer the Roaming
    // Throne's ward {2}. A seat that could not pay at all would be answering
    // a different question.
    let preset = table(
        &["Swords to Plowshares"],
        &["Plains", "Plains", "Plains"],
        &["Roaming Throne"],
    );
    let throne = entry("Roaming Throne").card;
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut answered = None;

    for seq in 0..400 {
        let pending = engine.pending();
        let Some(seat) = pending_player(pending) else {
            break;
        };
        // `awaiting` is carried over from the six-argument call this test was
        // written against: it named this seat as the awaited one, and
        // `SeatContext::default()` would quietly make it `None`. The agent
        // here answers the tax from `decision_context` rather than from the
        // view, so nothing observable turns on it today — which is the reason
        // to preserve it rather than to drop it, because a field nothing reads
        // is exactly the one a default silently changes.
        let ctx = SeatContext {
            awaiting: Some(seat),
            ..Default::default()
        };
        let view = player_view(engine.state(), seat, seq, Some(pending), &ctx);
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if matches!(
            pending,
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        ) {
            answered = Some(action.clone());
        }
        engine
            .apply(seat, action)
            .expect("every answer comes from the engine's own offer");
    }

    assert_eq!(
        answered,
        Some(PlayerAction::YesNo(true)),
        "the agent either never cast its removal at the warded creature, or \
         refused ward's tax with two untapped Plains and lost the spell — \
         which is what every tax got before `policy::pays_tax` read what \
         refusing one would do"
    );

    let state = engine.state();
    let survived = state
        .battlefield_view()
        .into_iter()
        .filter_map(|id| state.object(id))
        .any(|object| object.card.is_some_and(|card| card.index == throne));
    assert!(
        survived,
        "the Roaming Throne is gone, so the mana window after ward's question \
         was answered and Swords to Plowshares resolved. That is the outcome \
         this test is waiting for: `PlayerView` has learnt to say what a seat \
         owes. Delete this assertion and assert the resolution instead."
    );
}
