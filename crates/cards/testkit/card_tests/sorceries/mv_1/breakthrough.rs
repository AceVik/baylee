//! `cards/sorceries/mv_1/breakthrough.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn breakthrough() -> CardIndex {
    card_index("c6b26c1f-121f-4b7c-921e-429b623ca934")
}

/// Breakthrough (c6b26c1f-121f-4b7c-921e-429b623ca934) — {X}{U} Sorcery:
/// "Draw four cards, then choose X cards in your hand and discard the rest."
#[test]
fn breakthrough_draws_four_then_keeps_chosen_x_cards_and_discards_the_rest() {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), island()],
        &[],
        &[breakthrough(), sol_ring(), counterspell()],
        &[],
    );
    let ring = in_hand(&engine, P0, sol_ring()).unwrap();
    let spell = in_hand(&engine, P0, counterspell()).unwrap();
    let lib_before = library_size(&engine, P0);

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, breakthrough(), 0, 2, &[], &[]);
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
    assert_eq!(player, P0);
    assert_eq!(options.len(), 6, "2 starting cards + 4 drawn cards");
    assert_eq!(prompt, ChoicePrompt::Keep);
    assert_eq!((min, max), (2, 2), "choose X = 2 cards to keep");

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ring, spell],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(library_size(&engine, P0), lib_before - 4);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)),
        &[ring, spell]
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        5,
        "4 discarded cards + Breakthrough"
    );

    // X = 0 discards the entire hand after drawing 4.
    let mut zero = lk::game(SEED, &[island()], &[], &[breakthrough(), sol_ring()], &[]);
    tap_all_mana(&mut zero, P0);
    lk::announce(&mut zero, P0, breakthrough(), 0, 0, &[], &[]);
    lk::settle(&mut zero);
    assert!(zero.state().zones.list(ZoneLocation::Hand(P0)).is_empty());
}
