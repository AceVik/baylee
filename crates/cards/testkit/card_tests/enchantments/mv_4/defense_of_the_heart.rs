#[allow(clippy::wildcard_imports)]
use super::*;

fn defense_of_the_heart() -> CardIndex {
    card_index("e7e1b166-9267-426d-897d-24903327b48d")
}

#[test]
fn defense_of_the_heart_may_find_zero_one_or_two_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for count in 0..=2 {
        let mut engine = Duel::new(SEED, gray_ogre())
            .hand(0, &[defense_of_the_heart(), forest()])
            .battlefield(0, &[forest(), forest(), forest(), forest()])
            .battlefield(1, &[grizzly_bears(), gray_ogre(), earth_elemental()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        hand_to_library_top(&mut engine, p0, forest());
        cast_from_hand(&mut engine, p0, defense_of_the_heart());
        pass_until(&mut engine, stack_is_empty);
        reach_their_main_phase(&mut engine, p1);
        assert!(
            on_battlefield(&engine, p0, defense_of_the_heart()).is_some(),
            "only your upkeep triggers"
        );
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!(player, p0);
        assert_eq!(
            (min, max),
            (0, 2),
            "up to two, even with many matching cards"
        );
        assert!(
            options
                .iter()
                .all(|id| engine.state().object(*id).unwrap().card.unwrap().index == gray_ogre()),
            "noncreature cards are excluded"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options[..count].to_vec(),
                },
            )
            .unwrap();
        reach_their_main_phase(&mut engine, p0);
        assert!(
            in_graveyard(&engine, p0, defense_of_the_heart()).is_some(),
            "the enchantment was sacrificed even when finding none"
        );
        assert_eq!(all_on_battlefield(&engine, p0, gray_ogre()).len(), count);
    }
}

#[test]
fn defense_of_the_heart_does_not_add_creatures_from_different_opponents() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[defense_of_the_heart()])
        .battlefield(1, &[gray_ogre(), grizzly_bears()])
        .battlefield(2, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, defense_of_the_heart()).is_some(),
        "no opponent individually has three creatures"
    );
}
