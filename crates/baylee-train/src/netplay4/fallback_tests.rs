use super::*;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{NumberPrompt, timeout_answer};

#[test]
fn missing_timeout_default_never_turns_a_number_question_into_a_pass() {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let preset = crate::selfplay::table_for(42, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let session = baylee_gamehost::Session::new(&preset).unwrap();
    let view = session.observe_view(PlayerId::new(0));
    for (min, max, reason) in [(0, u32::MAX, NumberPrompt::X), (7, 9, NumberPrompt::Choice)] {
        let pending = Pending::ChooseNumber {
            player: view.seat,
            min,
            max,
            reason,
        };
        assert!(timeout_answer(&pending).is_none());
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let cands = encode_candidates4(&state, &view, &pending);
        let action = fallback_action(&cands, &view, &pending, &[]);
        assert!(
            pending.answer_fault(&action).is_none(),
            "{pending:?}: {action:?}"
        );
        assert!(matches!(action, PlayerAction::ChooseNumber(n) if (min..=max).contains(&n)));
    }
}
