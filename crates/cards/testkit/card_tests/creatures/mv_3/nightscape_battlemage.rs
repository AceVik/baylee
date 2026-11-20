//! `cards/creatures/mv_3/nightscape_battlemage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn nightscape_battlemage() -> CardIndex {
    card_index("e96e68b4-cb32-4b75-a885-e1781f4b74ab")
}

/// Casts Nightscape Battlemage off floating mana and answers `{2}{U}` (0)
/// and `{2}{R}` (1) in Oracle order.
fn cast_nightscape_battlemage(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_blue: bool,
    kick_red: bool,
) {
    cast_with_floating(engine, seat, nightscape_battlemage());
    let mut saw_0 = false;
    let mut saw_1 = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::KickerOption { index, cost, .. },
                ..
            } => {
                assert_eq!(player, seat);
                match index {
                    0 => {
                        assert!(!saw_0);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{2}{U}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_blue)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{2}{R}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_red)).unwrap();
                    }
                    other => panic!("unexpected kicker index {other}"),
                }
            }
            _ => break,
        }
    }
    assert!(saw_0 && saw_1, "both dual kicker options were offered");
}

#[test]
fn nightscape_battlemage_unkicked_skips_both_triggers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[nightscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS, plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_nightscape_battlemage(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p0, nightscape_battlemage()).is_some());
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, plains()).is_some());
}

#[test]
fn nightscape_battlemage_first_kicker_bounces_up_to_two_nonblack_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[nightscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::SCATHE_ZOMBIES, plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let zombies = on_battlefield(&engine, p1, index::SCATHE_ZOMBIES).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_nightscape_battlemage(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
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
    assert_eq!((min, max), (0, 2), "up to two target nonblack creatures");
    assert!(options.contains(&my_elf) && options.contains(&their_bear));
    assert!(
        !options.contains(&zombies),
        "black creature cannot be targeted: {options:?}"
    );
    let mage = on_battlefield(&engine, p0, nightscape_battlemage()).unwrap();
    assert!(
        !options.contains(&mage),
        "Nightscape Battlemage itself is black and cannot be targeted"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf, their_bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());
    assert!(in_hand(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(
        on_battlefield(&engine, p1, plains()).is_some(),
        "{{2}}{{R}} kicker was not paid, so land stays"
    );
}

#[test]
fn nightscape_battlemage_second_kicker_destroys_land_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[nightscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS, plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&engine, p1, plains()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_nightscape_battlemage(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&land));
    assert!(!options.contains(&bear));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, plains()).is_some());
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn nightscape_battlemage_both_kickers_bounce_and_destroy_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[nightscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS, plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&engine, p1, plains()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_nightscape_battlemage(&mut engine, p0, true, true);
    for _ in 0..20 {
        if at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { options, min, .. } => {
                if min == 0 {
                    assert!(options.contains(&bear));
                    engine
                        .apply(
                            p0,
                            PlayerAction::ChooseObjects {
                                objects: vec![bear],
                            },
                        )
                        .unwrap();
                } else {
                    assert!(options.contains(&land));
                    engine
                        .apply(
                            p0,
                            PlayerAction::ChooseObjects {
                                objects: vec![land],
                            },
                        )
                        .unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    assert!(in_hand(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, plains()).is_some());
}
