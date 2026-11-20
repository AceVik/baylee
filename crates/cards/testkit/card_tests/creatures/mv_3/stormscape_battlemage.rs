//! `cards/creatures/mv_3/stormscape_battlemage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn stormscape_battlemage() -> CardIndex {
    card_index("38ee748d-adcd-41df-9b23-d2a34829784c")
}

/// Casts Stormscape Battlemage off floating mana and answers its dual kicker
/// prompts `{W}` (index 0) and `{2}{B}` (index 1) in Oracle order.
fn cast_stormscape_battlemage(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_white: bool,
    kick_black: bool,
) {
    cast_with_floating(engine, seat, stormscape_battlemage());
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
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{W}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_white)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{2}{B}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_black)).unwrap();
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
fn stormscape_battlemage_unkicked_and_first_kicker_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // Unkicked: neither life gain nor destroy trigger stacks.
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[stormscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_stormscape_battlemage(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(engine.state().players[0].life, 20);
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some());

    // First kicker ({W}) only: gain 3 life, no destroy trigger.
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[stormscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_stormscape_battlemage(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(engine.state().players[0].life, 23);
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn stormscape_battlemage_second_kicker_destroys_nonblack_through_regen_shield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[stormscape_battlemage()])
        .battlefield(1, &[index::LIVING_WALL, index::SCATHE_ZOMBIES, island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p1, index::LIVING_WALL).unwrap();
    let zombies = on_battlefield(&engine, p1, index::SCATHE_ZOMBIES).unwrap();

    // Put a regeneration shield on Living Wall before casting Stormscape Battlemage.
    // Wait until p1 receives priority or pass once? Easier: cast Stormscape Battlemage
    // and let p1 activate Living Wall's {1}: Regenerate in response to the ETB trigger.
    tap_all_mana(&mut engine, p0);
    cast_stormscape_battlemage(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&wall), "nonblack creature is offered");
    assert!(
        !options.contains(&zombies),
        "black creature cannot be targeted: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();

    // Resolve down until p1 gets priority over the ETB destroy trigger, then shield Living Wall.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, index::LIVING_WALL, 0);
    pass_until(&mut engine, |e| {
        e.state()
            .object(wall)
            .is_some_and(|o| o.regeneration_shields == 1)
    });
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(engine.state().players[0].life, 20, "no {{W}} kicker paid");
    assert!(
        in_graveyard(&engine, p1, index::LIVING_WALL).is_some(),
        "Living Wall cannot be regenerated against Stormscape Battlemage's trigger"
    );
    assert!(on_battlefield(&engine, p1, index::SCATHE_ZOMBIES).is_some());
}

#[test]
fn stormscape_battlemage_both_kickers_gain_life_and_destroy_nonblack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[stormscape_battlemage()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_stormscape_battlemage(&mut engine, p0, true, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(engine.state().players[0].life, 23);
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
}
