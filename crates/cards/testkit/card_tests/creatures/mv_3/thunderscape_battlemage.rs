//! `cards/creatures/mv_3/thunderscape_battlemage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thunderscape_battlemage() -> CardIndex {
    card_index("9aa66e7d-d5e3-4c48-9c68-b61ff2af0b1a")
}

/// Casts Thunderscape Battlemage off floating mana and answers `{1}{B}` (0)
/// and `{G}` (1) in Oracle order.
fn cast_thunderscape_battlemage(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_black: bool,
    kick_green: bool,
) {
    cast_with_floating(engine, seat, thunderscape_battlemage());
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
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{1}{B}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_black)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{G}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_green)).unwrap();
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
fn thunderscape_battlemage_unkicked_skips_both_triggers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[thunderscape_battlemage()])
        .hand(1, &[plains(), plains(), plains()])
        .battlefield(1, &[their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_thunderscape_battlemage(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p0, thunderscape_battlemage()).is_some());
    assert_eq!(cards_in(&engine, ZoneLocation::Hand(p1)), 3);
    assert!(on_battlefield(&engine, p1, their_enchantment()).is_some());
}

#[test]
fn thunderscape_battlemage_first_kicker_makes_target_player_discard_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[thunderscape_battlemage()])
        .hand(1, &[plains(), island(), swamp()])
        .battlefield(1, &[their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_thunderscape_battlemage(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(player_options.contains(&p0) && player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

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
    assert_eq!(player, p1);
    assert_eq!((min, max), (2, 2));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: options.into_iter().take(2).collect(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(cards_in(&engine, ZoneLocation::Hand(p1)), 1);
    assert_eq!(cards_in(&engine, ZoneLocation::Graveyard(p1)), 2);
    assert!(on_battlefield(&engine, p1, their_enchantment()).is_some());
}

#[test]
fn thunderscape_battlemage_second_kicker_destroys_only_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[thunderscape_battlemage()])
        .hand(1, &[plains(), plains()])
        .battlefield(1, &[their_enchantment(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ench = on_battlefield(&engine, p1, their_enchantment()).unwrap();
    let rock = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_thunderscape_battlemage(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&ench));
    assert!(
        !options.contains(&rock),
        "artifact is not an enchantment: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ench],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, their_enchantment()).is_some());
    assert!(on_battlefield(&engine, p1, quiet_artifact()).is_some());
    assert_eq!(cards_in(&engine, ZoneLocation::Hand(p1)), 2);
}

#[test]
fn thunderscape_battlemage_both_kickers_discard_two_and_destroy_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[thunderscape_battlemage()])
        .hand(1, &[plains(), island(), swamp()])
        .battlefield(1, &[their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ench = on_battlefield(&engine, p1, their_enchantment()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_thunderscape_battlemage(&mut engine, p0, true, true);
    for _ in 0..20 {
        if at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { player_options, .. } => {
                if player_options.is_empty() {
                    engine
                        .apply(
                            p0,
                            PlayerAction::ChooseObjects {
                                objects: vec![ench],
                            },
                        )
                        .unwrap();
                } else {
                    engine
                        .apply(
                            p0,
                            PlayerAction::ChooseTargets {
                                objects: vec![],
                                players: vec![p1],
                            },
                        )
                        .unwrap();
                }
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!((min, max), (2, 2));
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: options.into_iter().take(2).collect(),
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    assert_eq!(cards_in(&engine, ZoneLocation::Hand(p1)), 1);
    assert!(in_graveyard(&engine, p1, their_enchantment()).is_some());
}
