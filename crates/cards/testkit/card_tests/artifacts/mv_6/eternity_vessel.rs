//! `cards/artifacts/mv_6/eternity_vessel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn eternity_vessel() -> CardIndex {
    card_index("c8fa1319-6623-4999-acb5-a5e15e6d868c")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn bolt_self(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
    let bolt = in_hand(engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn cast_vessel(engine: &mut Engine<RegistryLookup>, p0: PlayerId) -> ObjectId {
    let vc = in_hand(engine, p0, eternity_vessel()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: vc })
        .unwrap();
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, p0, eternity_vessel()).unwrap()
}

/// Eternity Vessel (c8fa1319-6623-4999-acb5-a5e15e6d868c) — {6} Artifact:
/// "This artifact enters with X charge counters on it, where X is your life
/// total."
/// "Landfall — Whenever a land you control enters, you may have your life total
/// become the number of charge counters on this artifact."
///
/// Enters with 17 charge counters at 17 life, ignores opponent's landfall, and
/// restores life from 14 back to 17 when accepted on your landfall.
#[test]
fn eternity_vessel_enters_with_life_counters_and_landfall_resets_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(
            0,
            &[
                lightning_bolt(),
                lightning_bolt(),
                eternity_vessel(),
                forest(),
            ],
        )
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    bolt_self(&mut engine, p0);
    assert_eq!(life(&engine, p0), 17);

    let vessel = cast_vessel(&mut engine, p0);
    assert_eq!(counters_on(&engine, vessel, CounterKind::Charge), 17);

    reach_their_main_phase(&mut engine, p1);
    let p1_land = in_hand(&engine, p1, forest()).unwrap();
    engine
        .apply(p1, PlayerAction::PlayLand { card: p1_land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life(&engine, p0),
        17,
        "opponent's land does NOT trigger Vessel"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    bolt_self(&mut engine, p0);
    assert_eq!(life(&engine, p0), 14);

    let land1 = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: land1 })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life(&engine, p0),
        17,
        "accepted Landfall restored life to 17"
    );
}

/// Declining Landfall keeps a higher life total, accepting lowers it to the
/// counter count, and destroying Eternity Vessel in response still uses LKI.
#[test]
fn eternity_vessel_decline_lower_and_lki_on_removal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                plains(),
                plains(),
                fountain_of_youth(),
            ],
        )
        .hand(0, &[eternity_vessel(), forest(), forest(), disenchant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(plains()));
    let vessel = cast_vessel(&mut engine, p0);
    assert_eq!(counters_on(&engine, vessel, CounterKind::Charge), 20);

    tap_all_mana(&mut engine, p0);
    let foy = on_battlefield(&engine, p0, fountain_of_youth()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: foy,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 21);

    let land1 = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: land1 })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life(&engine, p0),
        21,
        "declined lowering life from 21 to 20"
    );

    pass_until(&mut engine, |e| e.state().turn.active == PlayerId::new(1));
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    let land2 = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: land2 })
        .unwrap();
    tap_all_mana(&mut engine, p0);
    let dis = in_hand(&engine, p0, disenchant()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: dis })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![vessel],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(in_graveyard(&engine, p0, eternity_vessel()).is_some());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life(&engine, p0),
        20,
        "LKI charge counters (20) used after removal"
    );
}
