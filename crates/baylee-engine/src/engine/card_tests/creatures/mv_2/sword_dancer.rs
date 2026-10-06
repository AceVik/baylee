//! `cards/creatures/mv_2/sword_dancer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sword Dancer is a 1/2 for `{1}{W}` with exactly one line: "`{W}{W}`:
/// Target attacking creature gets −1/−0 until end of turn." The word the
/// card hangs on is *attacking* — so there is one **non**-
/// attacking creature under each player on the battlefield, and both must
/// stay out of the target list, while the only attacker is the whole list.
/// Two Plains pay the cost from a pool that was tapped beforehand
/// (`can_afford` reads the pool and not the untapped lands), and what the
/// −1/−0 did is read only on combat damage: an attacker with 0 power
/// takes no life from the defender, an unpumped 1/1 one.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn sword_dancer_shrinks_an_attacking_creature_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), sword_dancer(), festering_goblin()])
        .battlefield(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dancer = on_battlefield(&engine, p0, sword_dancer()).expect("the Sword Dancer is out");
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("my Goblin is out");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin is out");
    assert_eq!(pt(&engine, dancer), (1, 2), "a printed 1/2 before the pump");

    // Attacker declared: the Sword Dancer is the only one that will meet the
    // filter condition in a moment.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dancer, Defender::Player(p1))],
            },
        )
        .expect("the Dancer was one of the offered attackers");

    // The active player holds priority after the attack declaration
    // (`CR 508.2`) — and only there is "attacking creature" true at all.
    // `pass_until` instead of a bare `matches!`: it also skips a
    // block declare that would lie before damage assignment, without the
    // attacker ceasing to be an attacker.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains, two white — the Dancer and both Goblins make no mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dancer, 0)),
        "{{W}}{{W}} is floating, so the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sword_dancer(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target attacking creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        !options.contains(&goblin),
        "a creature that stayed home is no attacking creature: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "and the Goblin across the table is not attacking either, so it is no \
         more a legal target than mine: {options:?}"
    );
    assert_eq!(
        options,
        vec![dancer],
        "the one attacker in the game is the whole menu"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![dancer],
            },
        )
        .expect("the attacker was the only option the question enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 601.2h: the target is named first and the {{W}}{{W}} paid last"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, dancer),
        (0, 2),
        "−1/−0 on the creature that was named, and no toughness either"
    );
    assert_eq!(
        pt(&engine, goblin),
        (1, 1),
        "the static reaches the target and no other creature"
    );

    // And the missing point of power is the card's purpose: without the
    // −1/−0, p1 would be at 19.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the 0-power attacker dealt no combat damage"
    );
}
