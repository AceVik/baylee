//! Magmasaur pays its upkeep by removing a counter or sacrifices and explodes.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
fn magmasaur() -> CardIndex {
    card_index("da38bfdf-b4c8-4ffe-ad53-b42c317cae5f")
}

#[test]
fn upkeep_removes_a_counter_or_explodes_with_last_known_counter_count() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for mode in [0, 1] {
        let mut e = lk::game(
            SEED,
            &[index::GRIZZLY_BEARS],
            &[index::AIR_ELEMENTAL, index::HILL_GIANT],
            &[magmasaur()],
            &[],
        );
        lk::float(&mut e, p0, &[(ManaColor::Red, 5)]);
        cast_with_floating(&mut e, p0, magmasaur());
        lk::settle(&mut e);
        let source = on_battlefield(&e, p0, magmasaur()).unwrap();
        assert_eq!(
            e.state()
                .object(source)
                .unwrap()
                .counters
                .get(baylee_cards_dsl::CounterKind::P1P1),
            5
        );
        reach_their_main_phase(&mut e, p1);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
        lk::settle(&mut e);
        if mode == 0 {
            assert_eq!(
                e.state()
                    .object(source)
                    .unwrap()
                    .counters
                    .get(baylee_cards_dsl::CounterKind::P1P1),
                4
            );
            assert!(on_battlefield(&e, p0, magmasaur()).is_some());
            assert_eq!(e.state().players[0].life, 20);
            assert_eq!(e.state().players[1].life, 20);
        } else {
            assert!(in_graveyard(&e, p0, magmasaur()).is_some());
            assert_eq!(e.state().players[0].life, 15);
            assert_eq!(e.state().players[1].life, 15);
            assert!(in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some());
            assert!(in_graveyard(&e, p1, index::HILL_GIANT).is_some());
            assert!(
                on_battlefield(&e, p1, index::AIR_ELEMENTAL).is_some(),
                "flying creatures are excluded"
            );
        }
    }
}
