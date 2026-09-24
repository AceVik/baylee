//! Ward across the real engine boundary, and the payment window behind it.
//!
//! The unit test beside `policy::pays_tax` builds ward's question; this one
//! earns it. Ward is synthesised as a trigger on the **caster**
//! (`trigger.rs`, CR 702.21) and only arrives once the spell is on the stack
//! and its target is chosen, so nothing short of a game proves that the
//! agent casts into it, is asked, reads the alternative and answers.
//!
//! Saying yes with an empty pool is handed a mana window (CR 605.3a,
//! `actions.rs`), and the window is an ordinary priority round — which is
//! what hid it. The agent held priority over two untapped Plains with
//! nothing castable and passed, so it paid for a spell it then lost.
//! `policy::pay_owed` reads `PlayerView::owed` and taps toward the price,
//! and this file is where that is proved end to end rather than against a
//! constructed view.
//!
//! **This test used to pin the opposite outcome, and the pin was wired to
//! the wrong end.** Its header said the view "carries no outstanding
//! payment. It cannot, today — there is no field for it and nothing
//! projects the engine's `mana_window`", and it promised that "the day the
//! view says what a seat owes, the second assertion fails". That day was
//! `VIEW_VERSION` 24, under #92. `session.rs` has filled `owed` at four
//! sites since, through `view::owed_payment`, and the client has read it at
//! five with a test file of its own — and nothing here moved, because the
//! assertion measured the last **consumer** while the prose claimed a
//! **capability**. A pin is only a pin if the thing that lifts the
//! limitation is the thing that turns it red.
//!
//! The cost of that is invisible as a test result and is the reason this is
//! written out rather than quietly deleted: for as long as it stood, anyone
//! reading this file was told the view could not express an outstanding
//! payment, and was talked out of the field they had come looking for.
use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_engine::engine::Engine;
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view};

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
fn the_agent_pays_ward_and_its_removal_resolves() {
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
        // Both fields are load-bearing now and neither may be defaulted.
        // `owed` is the price the engine is waiting for, and `awaiting` is
        // what says the price is *this* seat's: `policy::pay_owed` refuses
        // to pay unless the two agree, so a `SeatContext::default()` here
        // would silently restore the behaviour this test exists to catch.
        let ctx = SeatContext {
            awaiting: Some(seat),
            owed: owed_payment(&engine),
            ..Default::default()
        };
        let view = player_view(engine.state(), seat, seq, Some(pending), &ctx, &[]);
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
        !survived,
        "the Roaming Throne is still on the battlefield, so Swords to \
         Plowshares never resolved: the agent agreed to ward's tax and then \
         passed in the payment window it was handed, which is worse than \
         having refused. `policy::pay_owed` reads `PlayerView::owed` and taps \
         toward the price — check that `SeatContext::owed` is fed here, \
         because a `None` there looks exactly like an agent that cannot pay."
    );
}
