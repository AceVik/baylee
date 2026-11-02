use super::*;

#[test]
fn foretell_round_trips_without_changing_existing_candidates() {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let preset = crate::selfplay::table_for(42, &[&deck, &deck],
        &[baylee_core::preset::AIProfile::STEADY; 2], &[]);
    let session = baylee_gamehost::Session::new(&preset).unwrap();
    let view = session.observe_view(PlayerId::new(0));
    let card = view.hand[0].id;
    let mut pending = Pending::Priority {
        player: view.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            suspendable: vec![card],
            ..Default::default()
        }),
    };
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let before = encode_candidates4(&state, &view, &pending);
    if let Pending::Priority { legal, .. } = &mut pending {
        legal.foretellable.push(card);
    }
    let after = encode_candidates4(&state, &view, &pending);
    assert_eq!(after.candidates.len(), before.candidates.len() + 1);
    for (old, new) in before.candidates.iter().zip(&after.candidates) {
        assert_eq!(serde_json::to_value(old).unwrap(), serde_json::to_value(new).unwrap());
    }
    let action = PlayerAction::Foretell { card };
    let label = encode_executed_action4(&after, &pending, &action).expect("training label");
    assert_eq!(usize::from(label.single_idx), before.candidates.len());
    assert_eq!(decode_action4(&after, &view, &pending, &label), Some(action));
    assert_ne!(after.candidates[1].feat, after.candidates[2].feat,
        "suspend and foretell on the same object remain distinct");
}

#[test]
fn a_vote_candidate_observes_the_public_tally() {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let preset = crate::selfplay::table_for(42, &[&deck, &deck],
        &[baylee_core::preset::AIProfile::STEADY; 2], &[]);
    let session = baylee_gamehost::Session::new(&preset).unwrap();
    let mut view = session.observe_view(PlayerId::new(0));
    let choices = [ObjectId::new(100, 0), ObjectId::new(101, 0)];
    let pending = Pending::ChooseCards {
        player: view.seat,
        options: choices.to_vec(),
        min: 1,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::Vote,
        total: None,
    };
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let before = encode_candidates4(&state, &view, &pending);
    view.ballots.push(baylee_view::Ballot { voter: PlayerId::new(1), choice: choices[1] });
    let after = encode_candidates4(&state, &view, &pending);
    assert_eq!(before.candidates.len(), 2);
    assert_eq!(after.candidates.len(), 2);
    for (old, new) in before.candidates.iter().zip(&after.candidates) {
        assert_eq!(old.action, new.action);
        assert!(new.feat[18] > old.feat[18], "every candidate knows a vote was cast");
        if new.action == Some(PlayerAction::ChooseObjects { objects: vec![choices[1]] }) {
            assert!(new.feat[16] > old.feat[16], "the chosen permanent has one more vote");
        } else {
            assert_eq!(new.feat[16], old.feat[16]);
        }
    }
}
