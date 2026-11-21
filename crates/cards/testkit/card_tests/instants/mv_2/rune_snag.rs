//! `cards/instants/mv_2/rune_snag.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "49bf3805-cc9e-4550-8a01-171282aa49c9"
fn rune_snag() -> CardIndex {
    card_index("49bf3805-cc9e-4550-8a01-171282aa49c9")
}

/// Rune Snag ({1}{U}): counters target spell unless its controller pays {2}
/// plus {2} for each Rune Snag in ALL graveyards (tested with 0 -> {2} paid,
/// then 2 across both graveyards -> {6} unpaid -> countered).
#[test]
fn rune_snag_scales_tax_by_two_per_copy_in_all_graveyards() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[rune_snag(), rune_snag()],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT, rune_snag()],
    );
    reach_their_main_phase(&mut engine, P1);

    // 1. 0 Rune Snags in graveyards -> tax is {2}. P1 floats {2}{G}{G}, pays {2}.
    lk::float(&mut engine, P1, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut engine, P1, index::GRIZZLY_BEARS);
    let bears_spell = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, rune_snag());
    lk::aim(&mut engine, &[bears_spell], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayTax { mana: 2 },
            ..
        } if *player == P1
    ));
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P1), 0, "paid exact {{2}} tax");
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some());

    // 2. Now 1 in P0's GY + put 1 in P1's GY = 2 total -> tax is 2 + 2*2 = {6}.
    hand_to_graveyard(&mut engine, P1, rune_snag());
    lk::float(&mut engine, P1, &[(ManaColor::Red, 9)]); // {3}{R} + 5 left (< 6!)
    cast_with_floating(&mut engine, P1, index::HILL_GIANT);
    let giant_spell = top_of_stack(&engine);
    assert_eq!(lk::pool_total(&engine, P1), 5);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, rune_snag());
    lk::aim(&mut engine, &[giant_spell], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayTax { mana: 6 },
            ..
        } if *player == P1
    ));
    // Answering true with only 5 mana cannot pay {6}: pool stays 5 and spell is countered.
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P1), 5, "short pool was not spent");
    assert!(
        in_graveyard(&engine, P1, index::HILL_GIANT).is_some(),
        "Hill Giant countered when P1 only had 5 mana against {{6}} tax"
    );
}
