#[allow(clippy::wildcard_imports)]
use super::*;

fn catastrophe() -> CardIndex {
    card_index("ed68160e-5e3d-44f2-ac7a-3749fda10119")
}

#[test]
fn catastrophe_destroys_all_creatures_when_creature_mode_chosen() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[catastrophe()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, catastrophe());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p0, plains()).is_some());
}

#[test]
fn catastrophe_land_choice_stops_regeneration_of_a_land_creature() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[catastrophe(), index::MENDING_TOUCH])
        .battlefield(
            0,
            &[
                mishra_s_factory(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                forest(),
            ],
        )
        .battlefield(1, &[plains(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    lk::float(
        &mut engine,
        p0,
        &[
            (ManaColor::White, 6),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
    );
    activate(&mut engine, p0, mishra_s_factory(), 1);
    pass_until(&mut engine, stack_is_empty);
    let factory = on_battlefield(&engine, p0, mishra_s_factory()).unwrap();
    lk::cast_at(&mut engine, p0, index::MENDING_TOUCH, &[factory], &[]);
    cast_with_floating(&mut engine, p0, catastrophe());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, mishra_s_factory()).is_some(),
        "land creatures cannot regenerate either"
    );
    assert!(on_battlefield(&engine, p0, forest()).is_none());
    assert!(on_battlefield(&engine, p1, plains()).is_none());
    assert!(on_battlefield(&engine, p0, baylee_core::generated::index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}
