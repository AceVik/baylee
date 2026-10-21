//! `cards/enchantments/mv_3/mesmeric_trance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);

fn mesmeric_trance() -> CardIndex {
    card_index("d1cc6979-d786-4035-963b-768ac8cef116")
}

/// Mesmeric Trance (d1cc6979-d786-4035-963b-768ac8cef116) — {1}{U}{U} Enchantment:
/// - "Cumulative upkeep {1}"
/// - "{U}, Discard a card: Draw a card."
#[test]
fn mesmeric_trance_loots_for_blue_and_a_discard_and_has_cumulative_upkeep_one() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[mesmeric_trance(), island(), island()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    let trance = on_battlefield(&engine, P0, mesmeric_trance()).unwrap();
    let discard_card = in_hand(&engine, P0, forest()).unwrap();
    let first_island = on_battlefield(&engine, P0, island()).unwrap();

    // Upkeep 1 (turn 1): cumulative upkeep {1} puts 1 age counter on Mesmeric Trance; P0 pays {1}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == first_island);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(trance)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // In P0's main phase, tap the second Island and activate "{U}, Discard a card: Draw a card."
    reach_main_phase(&mut engine, P0);
    tap_all_mana_but(&mut engine, P0, None);
    let lib_before = library_size(&engine, P0);
    activate(&mut engine, P0, mesmeric_trance(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![discard_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, forest()).is_some(),
        "the discarded card is in P0's graveyard"
    );
    assert_eq!(
        library_size(&engine, P0),
        lib_before - 1,
        "Mesmeric Trance drew a card from P0's library"
    );

    // Upkeep 2 (turn 3): 2nd age counter placed; P0 declines cumulative upkeep and sacrifices Mesmeric Trance.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, mesmeric_trance()).is_none()
            && in_graveyard(&engine, P0, mesmeric_trance()).is_some(),
        "Mesmeric Trance is sacrificed when cumulative upkeep is declined"
    );
}
