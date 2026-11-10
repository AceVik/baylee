//! Oblivion Stone preserves fate-marked permanents, then clears the marks.

use super::*;

fn oblivion_stone() -> CardIndex {
    card_index("2fd61c9f-44d2-48d7-a271-8e4208407cca")
}

#[test]
fn oblivion_stone_spares_marked_nonlands_and_removes_all_fate_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                oblivion_stone(),
                gray_ogre(),
                forest(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, oblivion_stone(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(ogre)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::FATE),
        1
    );

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, oblivion_stone(), 1);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&e, p0, forest()).is_some());
    assert!(in_graveyard(&e, p1, quiet_artifact()).is_some());
    assert_eq!(
        e.state()
            .object(ogre)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::FATE),
        0
    );
}
