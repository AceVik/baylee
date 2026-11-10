//! `cards/lands/utility/command_beacon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Command Beacon: "{T}: Add {C}." / "{T}, Sacrifice this land: Put your commander into your hand from the command zone."
/// Under `Coverage::Partial`, the command-zone return ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn command_beacon_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(119, forest())
        .battlefield(0, &[command_beacon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beacon = on_battlefield(&engine, p0, command_beacon()).expect("Beacon deployed");
    activate(&mut engine, p0, command_beacon(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, beacon));
}

#[test]
fn beacon_selects_its_owners_commander_and_retains_the_selection_during_replacement() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for keep_in_command in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::COMMAND_BEACON])
            .commander(
                0,
                &[index::SHEOLDRED_THE_APOCALYPSE, index::VENSER_SHAPER_SAVANT],
            )
            .commander(1, &[index::URZA_LORD_HIGH_ARTIFICER])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let mine = e.state().zones.list(ZoneLocation::Command(p0)).clone();
        let theirs = e.state().zones.list(ZoneLocation::Command(p1))[0];
        activate(&mut e, p0, index::COMMAND_BEACON, 1);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending()
        else {
            unreachable!()
        };
        assert_eq!(options, &mine);
        assert_eq!((*min, *max), (1, 1));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        let version = e.state().object(mine[1]).unwrap().version;
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine[1]],
            },
        )
        .unwrap();
        assert!(
            matches!(e.pending(),Pending::YesNo{prompt:YesNoPrompt::CommanderReplace{card,..},..}if *card==mine[1])
        );
        e.apply(p0, PlayerAction::YesNo(keep_in_command)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(mine[1]).unwrap().zone,
            if keep_in_command {
                Zone::Command
            } else {
                Zone::Hand
            }
        );
        if keep_in_command {
            assert_eq!(e.state().object(mine[1]).unwrap().version, version);
        }
        assert_eq!(e.state().object(mine[0]).unwrap().zone, Zone::Command);
    }
}
