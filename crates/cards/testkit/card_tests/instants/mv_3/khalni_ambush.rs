//! `cards/instants/mv_3/khalni_ambush.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Khalni Ambush // Khalni Territory: "Target creature you control fights
/// target creature you don't control. // This land enters tapped. {T}: Add
/// {G}."
///
/// The back face. The test plays it as a land, asserts that it enters
/// tapped, advances to the next turn so it untaps, and activates its mana
/// ability to add `{G}`; the front face is the test below.
#[test]
fn khalni_territory_enters_tapped_and_taps_for_green_mana() {
    let (mut engine, territory) =
        play_land_face(khalni_ambush(), 1).expect("plays as Khalni Territory");
    assert!(
        is_tapped(&engine, territory),
        "Khalni Territory enters tapped"
    );

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, territory), "untaps on next turn");
    let green_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);

    activate(&mut engine, p0, khalni_ambush(), 0);

    assert!(
        is_tapped(&engine, territory),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        green_before + 1,
        "adds one green mana to the pool"
    );
}

/// Khalni Ambush, the front face: "Target creature you control fights target
/// creature you don't control."
///
/// Two instances of the word "target", asked one after the other, each with
/// its own menu — the first holds only my creatures and the second only
/// theirs, which is the printing's "you control" / "you don't control". Then
/// a 4/4 fights a 2/2: each deals damage equal to its power to the other
/// (CR 701.14a), so the 2/2 dies and the 4/4 keeps two damage.
#[test]
fn khalni_ambush_makes_my_creature_fight_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4726, forest())
        .battlefield(0, &[forest(), forest(), forest(), fangren_hunter()])
        .battlefield(1, &[wild_colos()])
        .hand(0, &[khalni_ambush()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, khalni_ambush());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the first target question, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&hunter), "my creature is the fighter");
    assert!(
        !options.contains(&colos),
        "theirs is not \"a creature you control\""
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hunter],
                players: vec![],
            },
        )
        .expect("the Hunter was offered");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the second target question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![colos],
        "only a creature I don't control is the foe"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![colos],
                players: vec![],
            },
        )
        .expect("the Colos was offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wild_colos()).is_some(),
        "four damage kill the 2/2"
    );
    assert!(on_battlefield(&engine, p0, fangren_hunter()).is_some());
    assert_eq!(
        engine.state().object(hunter).map(|o| o.damage),
        Some(2),
        "and the 2/2 dealt its two back"
    );
    assert!(in_graveyard(&engine, p0, khalni_ambush()).is_some());
}

/// Misdirection's "target spell with a single target" (CR 115.9a): Khalni
/// Ambush holds two targets, one per instance of "target", so with only it
/// on the stack Misdirection has nothing to aim at and is not offered.
#[test]
fn misdirection_cannot_aim_at_a_spell_with_two_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4727, forest())
        .battlefield(0, &[forest(), forest(), forest(), fangren_hunter()])
        .battlefield(1, &[wild_colos()])
        .hand(0, &[khalni_ambush()])
        .hand(1, &[misdirection(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, khalni_ambush());
    for target in [hunter, colos] {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .expect("the fight's two creatures");
    }
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    assert!(!stack_is_empty(&engine), "the Ambush waits on the stack");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let card = in_hand(&engine, p1, misdirection()).expect("Misdirection is in hand");
    assert!(
        !legal.castable.contains(&card),
        "a spell with two targets is no target for Misdirection"
    );
}
