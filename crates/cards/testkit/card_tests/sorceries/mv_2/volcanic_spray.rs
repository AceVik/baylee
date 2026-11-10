//! `cards/sorceries/mv_2/volcanic_spray.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn volcanic_spray() -> CardIndex {
    card_index("dcf3cd72-3c3d-4847-83be-105966b0cd6a")
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, spray: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: spray })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// The ordinary {1}{R} cast and the identical flashback cost each deal exactly
/// one damage to both players and every ground creature on either side. Both
/// fliers remain at zero damage through both resolutions.
#[test]
fn volcanic_spray_hits_ground_creatures_and_players_twice_but_never_fliers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                index::HILL_GIANT,
                index::STORM_CROW,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT, index::STORM_CROW])
        .hand(0, &[volcanic_spray()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let own_giant = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    let their_giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let own_crow = on_battlefield(&engine, p0, index::STORM_CROW).unwrap();
    let their_crow = on_battlefield(&engine, p1, index::STORM_CROW).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, volcanic_spray());
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, p0), 19, "the caster is also a player");
    assert_eq!(life_of(&engine, p1), 19, "the opponent is also a player");
    assert_eq!(engine.state().object(own_giant).unwrap().damage, 1);
    assert_eq!(engine.state().object(their_giant).unwrap().damage, 1);
    assert_eq!(engine.state().object(own_crow).unwrap().damage, 0);
    assert_eq!(engine.state().object(their_crow).unwrap().damage, 0);
    let spray = in_graveyard(&engine, p0, volcanic_spray()).unwrap();

    start_flashback(&mut engine, p0, spray);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(spray).unwrap().zone, Zone::Stack);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, p0), 18, "flashback repeats one, not two");
    assert_eq!(life_of(&engine, p1), 18, "both players are hit again");
    assert_eq!(engine.state().object(own_giant).unwrap().damage, 2);
    assert_eq!(engine.state().object(their_giant).unwrap().damage, 2);
    assert_eq!(engine.state().object(own_crow).unwrap().damage, 0);
    assert_eq!(engine.state().object(their_crow).unwrap().damage, 0);
    assert_eq!(engine.state().object(spray).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, volcanic_spray()).is_none());
}

/// One red mana cannot pay either half of {1}{R}; the spell stays in the
/// graveyard and the mana remains unspent.
#[test]
fn volcanic_spray_flashback_is_not_offered_with_only_one_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[volcanic_spray()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spray = hand_to_graveyard(&mut engine, p0, volcanic_spray());
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&spray));
    assert!(!legal.payable.contains(&spray));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spray })
            .is_err()
    );
    assert_eq!(engine.state().object(spray).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}
