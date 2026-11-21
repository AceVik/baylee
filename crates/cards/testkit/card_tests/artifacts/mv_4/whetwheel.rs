//! `cards/artifacts/mv_4/whetwheel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn whetwheel() -> CardIndex {
    card_index("c5b0cd1c-6f9b-4b48-9430-af78731e1a45")
}

fn activate_mill(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    wheel: ObjectId,
    x: u32,
    target: PlayerId,
) {
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wheel,
                ability_index: 0,
            },
        )
        .expect("activate Whetwheel");
    engine.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn is_face_down(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(baylee_engine::object::Status::FACE_DOWN)
}

/// Whetwheel (c5b0cd1c-6f9b-4b48-9430-af78731e1a45) — {4} Artifact:
/// "{X}{X}, {T}: Target player mills X cards."
/// "Morph {3}"
///
/// With 5 mana available, {X}{X} caps X at 2 (paying 4 mana, leaving 1), and
/// mills exactly 2 cards from the targeted player; can also target self or X=0.
#[test]
fn whetwheel_double_x_mills_x_cards_from_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                whetwheel(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wheel = on_battlefield(&engine, p0, whetwheel()).unwrap();
    let p1_lib_before = engine.state().zones.list(ZoneLocation::Library(p1)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wheel,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber for X, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 2), "{{X}}{{X}} with 5 mana caps X at 2");
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, wheel));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).len(),
        p1_lib_before - 2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        2
    );

    // Next turn: activate with X = 0 targeting self (p0) -> 0 cards milled.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    let p0_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    activate_mill(&mut engine, p0, wheel, 0, p0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        p0_gy_before
    );
}

/// Morph {3}: cast face down as a 2/2 colorless creature for {3}, turn it face
/// up for {3} as a special action, and immediately activate {X}{X}, {T} as a
/// noncreature artifact on the same turn.
#[test]
fn whetwheel_morphs_face_down_and_turns_face_up_to_mill() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[whetwheel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast face down for {3} via CastModeKind::Disguise.
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, whetwheel()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    let mode_idx = choose_cast_kind(&engine, CastModeKind::Disguise);
    engine
        .apply(p0, PlayerAction::ChooseMode(mode_idx))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let wheel = on_battlefield(&engine, p0, whetwheel()).unwrap();
    assert!(is_face_down(&engine, wheel), "Whetwheel entered face down");
    assert_eq!(pt(&engine, wheel), (2, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);

    // Turn face up for Morph {3} (special action; does not use the stack).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wheel,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .expect("turn Whetwheel face up for {3}");
    assert!(!is_face_down(&engine, wheel), "Whetwheel is now face up");
    assert!(!types(&engine, wheel).contains(TypeSet::CREATURE));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    // Immediately activate {X}{X}, {T} with X = 1 using the remaining 2 mana.
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    activate_mill(&mut engine, p0, wheel, 1, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 1
    );
}
