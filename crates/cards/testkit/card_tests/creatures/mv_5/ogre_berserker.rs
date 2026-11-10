//! `cards/creatures/mv_5/ogre_berserker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fanatic of Rhonas's Ferocious — "{T}: Add {G}{G}{G}{G}. Activate only if
/// you control a creature with power 4 or greater." The same board with and
/// without a 4/2 Ogre Berserker beside it: the gate is closed on the Snake's
/// own power 1 and open on the Ogre's 4.
#[test]
fn fanatic_of_rhonas_makes_four_green_beside_a_creature_with_power_four() {
    let p0 = PlayerId::new(0);
    let offered = |board: &[CardIndex]| {
        let mut engine = Duel::new(307, forest()).battlefield(0, board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let snake = on_battlefield(&engine, p0, fanatic_of_rhonas()).expect("Fanatic deployed");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        (engine, legal.abilities.contains(&(snake, 1)))
    };

    let (_, alone) = offered(&[fanatic_of_rhonas()]);
    assert!(!alone, "power 1 does not open the gate");

    let (mut engine, beside) = offered(&[fanatic_of_rhonas(), ogre_berserker()]);
    assert!(beside, "a 4/2 does");
    activate(&mut engine, p0, fanatic_of_rhonas(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4
    );
}

/// Ogre Berserker is a 4/2 Ogre Berserker with haste under `Coverage::Implemented` costing {4}{R}.
/// Its printed haste reaches the permanent through the continuous layer system.
/// Under `CR 302.6`, haste permits the creature to attack during the combat phase of the turn it entered.
/// Its 4 power connects unblocked against the opponent, lowering their life total from 20 to 16.
#[test]
fn ogre_berserker_has_haste_and_attacks_the_turn_it_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[ogre_berserker()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, ogre_berserker());
    pass_until(&mut engine, stack_is_empty);

    let ogre = on_battlefield(&engine, p0, ogre_berserker()).expect("Ogre Berserker resolved");
    assert_eq!(pt(&engine, ogre), (4, 2), "printed body is 4/2");
    assert!(
        keywords(&engine, ogre).contains(KeywordSet::HASTE),
        "Ogre Berserker has haste"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on declare-attackers")
    };
    assert_eq!(player, p0, "active seat declares attackers");
    assert!(
        attackers.contains(&ogre),
        "haste allows attacking on the entry turn: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p1))],
            },
        )
        .expect("attack declared");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player: blocker_player,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected block choice, got {:?}", engine.pending())
    };
    engine
        .apply(
            blocker_player,
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().players[1].life != 20);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "four combat damage dealt to defending player"
    );
    assert!(
        on_battlefield(&engine, p0, ogre_berserker()).is_some(),
        "Ogre Berserker survives combat"
    );
}
