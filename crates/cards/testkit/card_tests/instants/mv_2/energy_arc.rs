//! `cards/instants/mv_2/energy_arc.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn energy_arc() -> CardIndex {
    card_index("959bcf99-3c9a-4f99-98b2-13514d3dac16")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// `Energy Arc` ({W}{U}, Instant) untaps any number of target creatures.
#[test]
fn energy_arc_untaps_multiple_target_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), grizzly_bears(), hill_giant()])
        .hand(0, &[energy_arc()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    engine.dev_state_mut(p0).unwrap().set_tapped(bears, true);
    engine.dev_state_mut(p0).unwrap().set_tapped(giant, true);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, energy_arc());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, bears));
    assert!(!is_tapped(&engine, giant));
}

/// "Any number of target creatures": only the chosen ones untap, whoever controls them, and a
/// tapped creature that is not chosen stays tapped.
#[test]
fn energy_arc_untaps_only_the_chosen_creatures_of_either_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), grizzly_bears(), hill_giant()])
        .battlefield(1, &[gray_ogre(), grizzly_bears()])
        .hand(0, &[energy_arc()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    for id in [bears, giant, ogre, their_bears] {
        engine.dev_state_mut(p0).unwrap().set_tapped(id, true);
    }

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, energy_arc());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, bears), "chosen");
    assert!(!is_tapped(&engine, ogre), "chosen, though the opponent's");
    assert!(is_tapped(&engine, giant), "not chosen");
    assert!(is_tapped(&engine, their_bears), "not chosen");
}

/// Player 1 attacks player 0 with a Hill Giant and a Gray Ogre, keeping a Grizzly Bears home.
/// Player 0's tapped Grizzly Bears is untapped by Energy Arc after the attack, and blocks the
/// Giant while player 0's Gray Ogre (not a target) blocks player 1's. Returns the engine at the
/// end of that combat.
fn energy_arc_combat() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), grizzly_bears(), gray_ogre()])
        .battlefield(1, &[hill_giant(), gray_ogre(), grizzly_bears()])
        .hand(0, &[energy_arc()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    engine.dev_state_mut(p0).unwrap().set_tapped(bears, true);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::DeclareAttackers
    });
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let their_ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (giant, Defender::Player(p0)),
                    (their_ogre, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(is_tapped(&engine, bears));
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, energy_arc());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, bears), "untapped in time to block");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    let my_ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant), (my_ogre, their_ogre)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    engine
}

/// "Prevent all combat damage that would be dealt to and dealt by those creatures this turn": the
/// targeted Bears and the Giant it blocked take and deal nothing, while the two Ogres, which were
/// not targeted, hit each other as usual.
#[test]
fn energy_arc_prevents_combat_damage_to_and_by_its_targets_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let engine = energy_arc_combat();

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the target survives");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant survives");
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        0,
        "nothing dealt to the Bears"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        0,
        "nothing dealt by the Bears"
    );
    assert!(
        in_graveyard(&engine, p0, gray_ogre()).is_some()
            && in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "the untargeted Ogres killed each other"
    );
    assert_eq!(engine.state().players[0].life, 20, "everything was blocked");
}

/// "This turn": in player 0's own turn the same Bears fights a plain Bears and both die.
#[test]
fn energy_arc_prevention_ends_with_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = energy_arc_combat();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_bears, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
