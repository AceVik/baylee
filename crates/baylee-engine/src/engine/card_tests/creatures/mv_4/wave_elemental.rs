//! `cards/creatures/mv_4/wave_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wave_elemental() -> CardIndex {
    card_index("d8eed5f7-8724-45c7-b6fc-dcfcc400f09a")
}

fn hill_giant() -> CardIndex {
    baylee_core::generated::index::HILL_GIANT
}

/// Wave Elemental (`{2}{U}{U}`, `Coverage::Implemented`): 2/3 Elemental with
/// `{U}, {T}, Sacrifice this creature: Tap up to three target creatures
/// without flying.`
#[test]
fn wave_elemental_sacrifices_to_tap_up_to_three_nonflying_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wave_elemental(), island()])
        .battlefield(1, &[grizzly_bears(), llanowar_elves(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let wave = on_battlefield(&engine, p0, wave_elemental()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wave,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear, elf, giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, wave_elemental()).is_some());
    assert!(is_tapped(&engine, bear));
    assert!(is_tapped(&engine, elf));
    assert!(is_tapped(&engine, giant));
}

/// The limits of the target group: "up to three" (zero to three, a fourth is
/// refused), "creatures without flying" (the Air Elemental is not on the menu
/// and cannot be named), and only the creatures named are tapped.
#[test]
fn wave_elemental_taps_only_the_chosen_creatures_and_never_a_flier_or_a_fourth() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wave_elemental(), island()])
        .battlefield(
            1,
            &[
                grizzly_bears(),
                llanowar_elves(),
                hill_giant(),
                festering_goblin(),
                air_elemental(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let wave = on_battlefield(&engine, p0, wave_elemental()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let goblin = on_battlefield(&engine, p1, festering_goblin()).unwrap();
    let flier = on_battlefield(&engine, p1, air_elemental()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wave,
                ability_index: 0,
            },
        )
        .unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 3), "\"up to three target creatures\"");
    for ground in [bear, elf, giant, goblin] {
        assert!(options.contains(&ground), "a creature without flying");
    }
    assert!(!options.contains(&flier), "the Air Elemental has flying");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bear, flier],
                },
            )
            .is_err(),
        "a flier cannot be named"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bear, elf, giant, goblin],
                },
            )
            .is_err(),
        "a fourth target is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear, giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, wave_elemental()).is_some());
    assert!(is_tapped(&engine, bear));
    assert!(is_tapped(&engine, giant));
    assert!(!is_tapped(&engine, elf), "not chosen, not tapped");
    assert!(!is_tapped(&engine, goblin), "not chosen, not tapped");
    assert!(!is_tapped(&engine, flier), "the flier was never a target");
}
