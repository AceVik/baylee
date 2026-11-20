//! `cards/creatures/mv_3/thornscape_battlemage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn thornscape_battlemage() -> CardIndex {
    card_index("8d4d6806-cb01-49e3-91cc-fb0e7f7a8684")
}

/// Casts Thornscape Battlemage off floating mana and answers `{R}` (0) and
/// `{W}` (1) in Oracle order.
fn cast_thornscape_battlemage(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_red: bool,
    kick_white: bool,
) {
    cast_with_floating(engine, seat, thornscape_battlemage());
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
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{R}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_red)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{W}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_white)).unwrap();
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
fn thornscape_battlemage_unkicked_skips_both_triggers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), mountain(), plains()])
        .hand(0, &[thornscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS, quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_thornscape_battlemage(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p0, thornscape_battlemage()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, quiet_artifact()).is_some());
}

#[test]
fn thornscape_battlemage_first_kicker_deals_two_to_any_target_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), mountain(), plains()])
        .hand(0, &[thornscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS, quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_thornscape_battlemage(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert!(options.contains(&bear));
    assert!(player_options.contains(&p0) && player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, quiet_artifact()).is_some());
}

#[test]
fn thornscape_battlemage_second_kicker_destroys_only_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), mountain(), plains()])
        .hand(0, &[thornscape_battlemage()])
        .battlefield(1, &[their_enchantment(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ench = on_battlefield(&engine, p1, their_enchantment()).unwrap();
    let rock = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_thornscape_battlemage(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&rock));
    assert!(
        !options.contains(&ench),
        "enchantment is not an artifact: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
    assert!(on_battlefield(&engine, p1, their_enchantment()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn thornscape_battlemage_both_kickers_damage_player_and_destroy_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), mountain(), plains()])
        .hand(0, &[thornscape_battlemage()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rock = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_thornscape_battlemage(&mut engine, p0, true, true);
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
                                objects: vec![rock],
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
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
}
