//! `cards/creatures/mv_4/blastoderm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Blastoderm ({2}{G}{G} 5/5 Creature — Beast):
/// "Shroud; Fading 3 (This creature enters with three fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)"
#[test]
fn blastoderm_has_shroud_and_fades_away_after_three_upkeeps() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[card_index("d6ee8bd6-0207-46ee-9892-c1d1b23b934e")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(
        &mut engine,
        P0,
        card_index("d6ee8bd6-0207-46ee-9892-c1d1b23b934e"),
    );
    pass_until(&mut engine, stack_is_empty);

    let derm = on_battlefield(
        &engine,
        P0,
        card_index("d6ee8bd6-0207-46ee-9892-c1d1b23b934e"),
    )
    .unwrap();
    assert!(
        engine
            .state()
            .object(derm)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHROUD)
    );
    assert_eq!(
        engine
            .state()
            .object(derm)
            .unwrap()
            .counters
            .get(counters::FADE),
        3
    );

    for expected in [2, 1, 0] {
        reach_their_main_phase(&mut engine, P1);
        reach_their_main_phase(&mut engine, P0);
        assert_eq!(
            engine
                .state()
                .object(derm)
                .unwrap()
                .counters
                .get(counters::FADE),
            expected
        );
    }

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        on_battlefield(
            &engine,
            P0,
            card_index("d6ee8bd6-0207-46ee-9892-c1d1b23b934e")
        )
        .is_none()
    );
    assert!(
        in_graveyard(
            &engine,
            P0,
            card_index("d6ee8bd6-0207-46ee-9892-c1d1b23b934e")
        )
        .is_some()
    );
}
