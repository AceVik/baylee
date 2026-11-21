//! `cards/instants/mv_3/oppressive_will.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "983a2a50-3634-4783-8576-30916c3d0818"
fn oppressive_will() -> CardIndex {
    card_index("983a2a50-3634-4783-8576-30916c3d0818")
}

/// Oppressive Will ({2}{U}): counters target spell unless its controller pays
/// {1} for each card in YOUR hand on resolution (excluding Oppressive Will
/// itself on the stack).
#[test]
fn oppressive_will_taxes_by_your_hand_size_to_counter_or_allow() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[oppressive_will(), oppressive_will(), island(), island()],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
    );
    reach_their_main_phase(&mut engine, P1);

    // 1. P0 holds 3 other cards after casting -> tax is {3}. P1 only has 2 left -> countered.
    lk::float(&mut engine, P1, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut engine, P1, index::GRIZZLY_BEARS);
    let bears = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut engine, P0, oppressive_will());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayTax { mana: 3 },
            ..
        } if *player == P1
    ));
    // With only 2 mana in pool, attempting to pay {3} fails and counters Bears.
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(
        lk::pool_total(&engine, P1),
        2,
        "unspent {{G}}{{G}} stayed in pool"
    );
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());

    // 2. P0 now holds 2 cards after casting second copy -> tax is {2}.
    //    P1 adds {R}{R}{R}{R} to existing {G}{G} (= 6), casts {3}{R} Giant, leaves {2}.
    lk::float(&mut engine, P1, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut engine, P1, index::HILL_GIANT);
    let giant = top_of_stack(&engine);
    assert_eq!(lk::pool_total(&engine, P1), 2);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut engine, P0, oppressive_will());
    lk::aim(&mut engine, &[giant], &[]);
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
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_some());
}
