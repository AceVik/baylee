//! `cards/creatures/mv_4/body_snatcher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn body_snatcher() -> CardIndex {
    card_index("e04688e0-672b-407f-a2c9-fd7292926483")
}

#[test]
fn body_snatcher_requires_a_creature_discard_and_reanimates_on_death() {
    use crate::engine::card_tests::legends_kit as lk;
    for pay in [false, true] {
        let mut engine = lk::game(
            SEED,
            &[],
            &[],
            &[body_snatcher(), llanowar_elves(), forest()],
            &[],
        );
        lk::float(&mut engine, lk::P0, &[(ManaColor::Black, 4)]);
        let elf = in_hand(&engine, lk::P0, llanowar_elves()).unwrap();
        let land = in_hand(&engine, lk::P0, forest()).unwrap();
        cast_with_floating(&mut engine, lk::P0, body_snatcher());
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = pass_to_card_choice(&mut engine)
        else {
            unreachable!()
        };
        assert_eq!(player, lk::P0);
        assert_eq!((min, max), (0, 1));
        assert!(options.contains(&elf));
        assert!(!options.contains(&land));
        engine
            .apply(
                lk::P0,
                PlayerAction::ChooseObjects {
                    objects: if pay { vec![elf] } else { vec![] },
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        if !pay {
            let snatcher = engine
                .state()
                .zones
                .list(ZoneLocation::Exile(lk::P0))
                .iter()
                .copied()
                .find(|id| {
                    engine
                        .state()
                        .object(*id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index == body_snatcher()))
                });
            assert!(snatcher.is_some(), "declining the discard exiles it");
            continue;
        }
        let snatcher = lk::obj(&engine, lk::P0, body_snatcher());
        let elf = in_graveyard(&engine, lk::P0, llanowar_elves()).unwrap();
        bury(&mut engine, &[snatcher]);
        lk::pass_once(&mut engine);
        let Pending::ChooseTargets { options, .. } = engine.pending() else {
            panic!("dies trigger needs its target")
        };
        assert!(options.contains(&elf));
        lk::aim(&mut engine, &[elf], &[]);
        pass_until(&mut engine, stack_is_empty);
        assert!(on_battlefield(&engine, lk::P0, llanowar_elves()).is_some());
        assert!(in_graveyard(&engine, lk::P0, body_snatcher()).is_none());
        assert!(on_battlefield(&engine, lk::P0, body_snatcher()).is_none());
    }
}

#[test]
fn body_snatcher_exiles_when_there_is_no_creature_to_discard() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[body_snatcher(), forest()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut engine, lk::P0, body_snatcher());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, body_snatcher()).is_none());
    assert!(in_graveyard(&engine, lk::P0, body_snatcher()).is_none());
}

#[test]
fn body_snatcher_may_target_itself_but_does_not_return_after_exile() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[body_snatcher()], &[], &[], &[]);
    let snatcher = lk::obj(&engine, lk::P0, body_snatcher());
    bury(&mut engine, &[snatcher]);
    lk::pass_once(&mut engine);
    let grave = in_graveyard(&engine, lk::P0, body_snatcher()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("death target")
    };
    assert!(
        options.contains(&grave),
        "its own graveyard card is a legal target"
    );
    lk::aim(&mut engine, &[grave], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, body_snatcher()).is_none());
    assert!(in_graveyard(&engine, lk::P0, body_snatcher()).is_none());
}
