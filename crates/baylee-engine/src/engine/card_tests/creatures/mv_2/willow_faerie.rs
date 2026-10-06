//! `cards/creatures/mv_2/willow_faerie.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Willow Faerie is a `{1}{G}` 1/2 whose entire printed text is Flying, so a
/// game can only show the card by casting it and then swinging with it: the
/// keyword has no trigger and no cost that a state read could catch. Two
/// Forests are exactly the `{1}` and the `{G}`, and the offer is read while
/// that mana is already floating, because `can_afford` reads the pool and not
/// the untapped lands. A turn later the Faerie attacks over the untapped
/// Llanowar Elves standing across the table and takes a life off a player
/// whose only creature is on the ground — an evasive body the test actually
/// turned sideways, with a real blocker present so the damage is not proven
/// by an empty board.
#[test]
fn willow_faerie_lands_as_a_flying_one_two_and_attacks_over_the_ground() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[willow_faerie()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, two green, and no creature beside them that could have paid"
    );
    let card = in_hand(&engine, p0, willow_faerie()).expect("the Faerie is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "{{1}}{{G}} is affordable off the two Forests: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, willow_faerie());
    pass_until(&mut engine, stack_is_empty);

    let faerie = on_battlefield(&engine, p0, willow_faerie()).expect("the Faerie resolved");
    assert_eq!(pt(&engine, faerie), (1, 2), "the printed 1/2 body");
    assert!(
        keywords(&engine, faerie).contains(KeywordSet::FLYING),
        "and `Flying`, read through the layers rather than off the card file"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} came out of the pool the two Forests filled"
    );

    // A whole turn cycle, so the Faerie is no longer summoning sick (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert!(
        attackers.contains(&faerie),
        "an untapped creature that has been on the table since last turn may \
         attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(faerie, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Past the combat damage step (CR 510.2): the Elf across the table is a
    // ground creature and no blocker for a flyer, so it is the control that
    // keeps this from being an attack into nothing.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "one power of flying damage got through the Llanowar Elves"
    );
    assert_eq!(
        pt(&engine, faerie),
        (1, 2),
        "and the attacker is untouched, because nothing was able to block it"
    );
}
