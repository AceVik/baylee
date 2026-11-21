//! `cards/instants/mv_3/waylay.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f0db78c7-09b5-4981-8ca8-6dbab597c5d9"
fn waylay() -> CardIndex {
    card_index("f0db78c7-09b5-4981-8ca8-6dbab597c5d9")
}

/// Waylay ({2}{W}): creates three 2/2 white Knight creature tokens, keeps them
/// through the end step, and exiles all three at the beginning of the next
/// cleanup step.
#[test]
fn waylay_creates_three_white_knights_and_exiles_them_at_cleanup() {
    let mut engine = lk::game(SEED, &[], &[], &[waylay()], &[]);

    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut engine, P0, waylay());
    lk::settle(&mut engine);

    let knights = tokens_of(&engine, P0);
    assert_eq!(knights.len(), 3, "three Knight tokens created");
    for &knight in &knights {
        assert_eq!(pt(&engine, knight), (2, 2), "2/2 Knight token");
        let ch = engine.state().object(knight).unwrap().characteristics();
        assert!(ch.colors.contains(Color::White), "white token");
        assert!(
            ch.subtypes
                .contains(baylee_core::generated::subtypes::creature::KNIGHT),
            "Knight subtype: {:?}",
            ch.subtypes
        );
    }

    // Tokens are still on the battlefield during the End step (not exiled early).
    lk::to_step(&mut engine, P0, Step::End);
    assert_eq!(
        tokens_of(&engine, P0).len(),
        3,
        "Waylay exiles at cleanup, not at the beginning of the end step"
    );

    // Passing into P1's turn crosses the cleanup step, exiling all three tokens.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        tokens_of(&engine, P0).is_empty(),
        "all three Knight tokens were exiled at the cleanup step"
    );
}
