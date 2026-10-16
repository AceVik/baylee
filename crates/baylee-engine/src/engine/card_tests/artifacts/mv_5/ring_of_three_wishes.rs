//! `cards/artifacts/mv_5/ring_of_three_wishes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ring_of_three_wishes() -> CardIndex {
    card_index("677c3616-295f-493a-8b60-1e22b0224cfa")
}

/// "This artifact enters with three wish counters on it."
#[test]
fn it_enters_with_three_wish_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[ring_of_three_wishes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, ring_of_three_wishes());
    pass_until(&mut engine, stack_is_empty);
    let ring = on_battlefield(&engine, p0, ring_of_three_wishes()).unwrap();
    assert_eq!(
        counters_on(&engine, ring, baylee_cards_dsl::counters::WISH),
        3
    );
}

/// "{5}, {T}, Remove a wish counter from this artifact: Search your library
/// for a card, put that card into your hand, then shuffle." Any card; one
/// counter goes.
#[test]
fn a_wish_fetches_a_card_of_my_choice_into_my_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ring_of_three_wishes(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, ring_of_three_wishes()).unwrap();
    set_counters(&mut engine, ring, baylee_cards_dsl::counters::WISH, 3);
    let lib = library_size(&engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ring_of_three_wishes(), 0);
    assert!(is_tapped(&engine, ring));
    assert_eq!(
        counters_on(&engine, ring, baylee_cards_dsl::counters::WISH),
        2,
        "the counter is a cost"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(
        (min, max),
        (1, 1),
        "an unrestricted search must find a card when the library is non-empty"
    );
    assert!(
        !options.is_empty(),
        "the library is searchable: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a listed card");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), lib - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "the card went to the hand"
    );
}

/// With no counter left the ability cannot be paid.
#[test]
fn without_wish_counters_it_is_not_offered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ring_of_three_wishes(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, ring_of_three_wishes()).unwrap();
    set_counters(&mut engine, ring, baylee_cards_dsl::counters::WISH, 0);
    tap_all_mana(&mut engine, p0);
    assert!(!priority_offer(&engine).abilities.contains(&(ring, 0)));
    set_counters(&mut engine, ring, baylee_cards_dsl::counters::WISH, 1);
    assert!(priority_offer(&engine).abilities.contains(&(ring, 0)));
}
