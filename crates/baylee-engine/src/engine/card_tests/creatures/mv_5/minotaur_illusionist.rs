//! Oracle scenarios for Minotaur Illusionist.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn minotaur_illusionist() -> CardIndex {
    card_index("ee5fc6e9-0c10-4015-8853-dd1f33dcc27e")
}

#[test]
fn minotaur_illusionist_gains_shroud_and_sacrifices_to_deal_power_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::MOUNTAIN,
                index::MOUNTAIN,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[minotaur_illusionist()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let giant = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    cast_from_hand(&mut e, p0, minotaur_illusionist());
    pass_until(&mut e, stack_is_empty);

    let minotaur = on_battlefield(&e, p0, minotaur_illusionist()).unwrap();
    tap_all_mana(&mut e, p0);

    // {1}{U}: Gain shroud until end of turn.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: minotaur,
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(minotaur)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHROUD)
    );

    // {R}, Sacrifice this creature: Deal damage equal to its power (3) to target creature.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: minotaur,
            ability_index: 1,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    assert!(in_graveyard(&e, p0, minotaur_illusionist()).is_some());
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p1, index::HILL_GIANT).is_some());
}

#[test]
fn minotaur_illusionist_uses_pumped_lki_power_when_sacrificed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::FOREST,
            ],
        )
        .battlefield(1, &[index::MAHAMOTI_DJINN])
        .hand(0, &[minotaur_illusionist(), index::GIANT_GROWTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let djinn = on_battlefield(&e, p1, index::MAHAMOTI_DJINN).unwrap();
    cast_from_hand(&mut e, p0, minotaur_illusionist());
    pass_until(&mut e, stack_is_empty);

    let minotaur = on_battlefield(&e, p0, minotaur_illusionist()).unwrap();
    cast_from_hand(&mut e, p0, index::GIANT_GROWTH);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![minotaur],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, minotaur), (6, 7));

    tap_all_mana(&mut e, p0);
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: minotaur,
            ability_index: 1,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![djinn],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, p1, index::MAHAMOTI_DJINN).is_some(),
        "6 LKI power destroyed the 5/6 Mahamoti Djinn"
    );
}
