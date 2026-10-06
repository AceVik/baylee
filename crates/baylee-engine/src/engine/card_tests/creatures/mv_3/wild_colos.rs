//! `cards/creatures/mv_3/wild_colos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wild Colos is a `{2}{R}` 2/2 Goat Beast whose printed text is the single
/// word "Haste", and haste is a rule about *this* turn: CR 702.10 lets the
/// creature attack the turn it arrived, which is the only moment it differs
/// from an identical body that does not print it. So both creatures are cast
/// in one main phase off four lands and read at the same attack declaration —
/// the Colos is offered and the Llanowar Elves cast beside it is not, which
/// only the keyword can produce, since both arrived under the same controller
/// in the same step (CR 302.6). Declaring the offered attacker and following
/// the two damage to the defending seat is that offer taken rather than
/// assumed, and `(2, 2)` is the body the card prints.
#[test]
fn wild_colos_attacks_the_turn_it_arrives_where_a_body_without_haste_cannot() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), forest()])
        .hand(0, &[wild_colos(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `{2}{R}` and `{G}` are paid out of one pool: three Mountains and a
    // Forest are every mana source the board has, and a pool survives until
    // the step ends (CR 500.5), so the four read here is the whole of what
    // the two casts below have to spend.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "three Mountains and one Forest, and nothing else on the board taps"
    );

    cast_with_floating(&mut engine, p0, wild_colos());
    pass_until(&mut engine, stack_is_empty);
    let colos = on_battlefield(&engine, p0, wild_colos()).expect("the Colos resolved");
    assert_eq!(pt(&engine, colos), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, colos).contains(KeywordSet::HASTE),
        "Haste is the card's whole printed text"
    );

    // The control arrives in the same main phase off the green the Colos left
    // behind, and is summoning sick for exactly one reason: it prints no
    // haste (CR 302.6).
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves resolved");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "no continuous effect on this board granted the Elves a keyword"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&colos),
        "\"Haste\" is what puts it here on the turn it was cast: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elves),
        "a creature that entered this turn without haste is not an attacker \
         (CR 302.6): {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(colos, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        18,
        "the offered attack happened: two power reached the defending seat"
    );
    assert!(
        on_battlefield(&engine, p0, wild_colos()).is_some(),
        "and the Colos is still the permanent it attacked with"
    );
}
