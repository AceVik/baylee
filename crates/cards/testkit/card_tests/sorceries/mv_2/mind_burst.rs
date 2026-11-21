//! `cards/sorceries/mv_2/mind_burst.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_burst() -> CardIndex {
    card_index("efc32c4a-ad84-400d-b79f-5861a3e4fa2f")
}

/// Mind Burst (efc32c4a-ad84-400d-b79f-5861a3e4fa2f) — {1}{B} Sorcery:
/// "Target player discards X cards, where X is one plus the number of cards
/// named Mind Burst in all graveyards."
#[test]
fn mind_burst_discards_one_plus_copies_named_mind_burst_in_all_graveyards() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp()],
        &[],
        &[mind_burst(), mind_burst()],
        &[mind_burst(), forest(), island(), plains(), mountain()],
    );
    hand_to_graveyard(&mut engine, P0, mind_burst());
    hand_to_graveyard(&mut engine, P1, mind_burst());

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, mind_burst(), 0, 0, &[], &[P1]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, P1);
    assert_eq!(options.len(), 4);
    assert_eq!(prompt, ChoicePrompt::Keep);
    assert_eq!(
        (min, max),
        (1, 1),
        "1 + 2 copies in graveyards = 3 discarded -> 4 - 3 = 1 kept"
    );
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P1)).len(), 1);
}
