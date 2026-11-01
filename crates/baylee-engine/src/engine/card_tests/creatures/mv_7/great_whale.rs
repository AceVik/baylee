#[allow(clippy::wildcard_imports)]
use super::*;

fn great_whale() -> CardIndex {
    card_index("c43ce30a-0346-479d-8dbc-56c3872ff82a")
}

#[test]
fn great_whale_untaps_zero_or_seven_chosen_lands_but_not_eight() {
    for count in [0, 7] {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(SEED, island())
            .hand(0, &[great_whale()])
            .battlefield(
                0,
                &[
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    grizzly_bears(),
                ],
            )
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let lands = all_on_battlefield(&engine, p0, island());
        let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
        cast_from_hand(&mut engine, p0, great_whale());
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = engine.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!((min, max), (0, 7));
        assert!(!options.contains(&bear));
        assert!(lands.iter().all(|land| options.contains(land)));
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: lands.clone()
                    }
                )
                .is_err()
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: lands.iter().take(count).copied().collect(),
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            lands
                .iter()
                .filter(|land| !is_tapped(&engine, **land))
                .count(),
            count
        );
    }
}
