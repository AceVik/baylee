//! `cards/instants/mv_4/grab_the_reins.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn grab_the_reins() -> CardIndex {
    card_index("3ceda405-8175-416a-beda-9a95f5acc60c")
}

/// Grab the Reins — {3}{R} Instant
/// "Choose one —
/// • Until end of turn, you gain control of target creature and it gains haste.
/// • Sacrifice a creature. Grab the Reins deals damage equal to that creature's power to any target.
/// Entwine {2}{R}"
#[test]
fn grab_the_reins_entwined_steals_creature_then_sacrifices_it_to_deal_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[grab_the_reins()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, grab_the_reins());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let entwine_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Modes(3))
        .expect("entwine mode offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(entwine_idx))
        .unwrap();

    // Target 1 (Mode 0: creature to steal): giant
    aim_at(&mut engine, p0, giant);
    // Target 2 (Mode 1: any target for damage): P1
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    // Wait for Mode 1's sacrifice prompt (since Giant is now controlled by P0 from Mode 0).
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, HILL_GIANT).is_none());
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn grab_the_reins_first_mode_grants_control_and_haste_until_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[grab_the_reins()])
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, grab_the_reins());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let mode = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(0))
        .expect("first mode offered alone");
    engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().controller, p0);
    assert!(keywords(&engine, giant).contains(KeywordSet::HASTE));
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().object(giant).unwrap().controller, p1);
    assert!(!keywords(&engine, giant).contains(KeywordSet::HASTE));
}
