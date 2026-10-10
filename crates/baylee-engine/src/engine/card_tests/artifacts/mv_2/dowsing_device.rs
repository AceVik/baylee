//! `cards/artifacts/mv_2/dowsing_device.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Entry pumps a controlled creature; fewer than four artifacts cannot transform the Device.
#[test]
fn dowsing_device_triggers_on_entry_to_pump_and_grant_haste_to_controlled_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(146, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[dowsing_device()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls an elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 controls an elf");

    assert_eq!(pt(&engine, my_elf), (1, 1), "elf starts as a 1/1");
    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::HASTE),
        "elf does not have haste yet"
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, dowsing_device());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&my_elf),
        "controlled creature is a legal target"
    );
    assert!(
        !options.contains(&their_elf),
        "opponent's creature is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_elf),
        (2, 1),
        "controlled creature received +1/+0 pump"
    );
    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::HASTE),
        "controlled creature gained haste"
    );

    let device = on_battlefield(&engine, p0, dowsing_device()).expect("device on battlefield");
    assert_eq!(
        engine.state().object(device).map(|o| o.face_index),
        Some(0),
        "fewer than four artifacts leave it on face 0"
    );
}

#[test]
fn dowsing_transforms_without_a_target_but_not_with_an_illegal_chosen_target() {
    use crate::engine::card_tests::legends_kit as kit;
    for invalid_target in [false, true] {
        let mut e = kit::game(
            976,
            &[
                sol_ring(),
                chromatic_lantern(),
                quiet_artifact(),
                llanowar_elves(),
            ],
            &[],
            &[dowsing_device()],
            &[],
        );
        let elf = kit::obj(&e, kit::P0, llanowar_elves());
        kit::float(&mut e, kit::P0, &[(ManaColor::Red, 2)]);
        cast_with_floating(&mut e, kit::P0, dowsing_device());
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        e.apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: if invalid_target { vec![elf] } else { vec![] },
            },
        )
        .unwrap();
        let device = kit::obj(&e, kit::P0, dowsing_device());
        let original = identity(&e, device);
        if invalid_target {
            e.state
                .move_object(
                    elf,
                    ZoneLocation::Graveyard(kit::P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(identity(&e, device), original);
        assert_eq!(
            e.state.object(device).unwrap().face_index,
            u8::from(!invalid_target)
        );
        if !invalid_target {
            kit::float(&mut e, kit::P0, &[(ManaColor::Red, 3)]);
            kit::use_ability(&mut e, kit::P0, dowsing_device(), 1, &[elf], &[]);
            assert_eq!(pt(&e, elf), (4, 1));
            assert!(keywords(&e, elf).contains(KeywordSet::HASTE));
        }
    }
}
