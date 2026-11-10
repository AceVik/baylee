//! `cards/creatures/mv_3/wall_of_air.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Air prints two keywords on a 1/5 body — Defender and flying — and
/// each is read off a different part of the engine: flying from the keyword
/// projection the layers produce, Defender from the combat step's own offer.
/// The Wall is really cast for {1}{U}{U} off three Islands, and a whole turn
/// cycle is walked before the attack declaration, because a creature that
/// arrived this turn is refused as an attacker by summoning sickness
/// (CR 302.6) whatever it prints — it is asserted untapped and past its
/// controller's untap step first, so its absence from `attackers` can only be
/// the word Defender. The Llanowar Elves standing beside it is the control
/// that says the offer was there to be missed.
#[test]
fn wall_of_air_arrives_as_a_flying_wall_the_combat_step_never_offers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .hand(0, &[wall_of_air()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{U}{U} off the three Islands, with the Elf's own {G} beside them in
    // the pool: the Wall arrives by being cast, not by being placed.
    cast_from_hand(&mut engine, p0, wall_of_air());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, wall_of_air()).expect("the Wall resolved");
    assert_eq!(pt(&engine, wall), (1, 5), "the body the card prints");

    let printed = keywords(&engine, wall);
    assert!(
        printed.contains(KeywordSet::FLYING),
        "flying reaches the permanent"
    );
    assert!(
        printed.contains(KeywordSet::DEFENDER),
        "and so does Defender"
    );

    // A full turn cycle, so the Wall is no longer summoning sick. Without this
    // the next claim would be satisfied by CR 302.6 rather than by the card's
    // own word.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "back to the Wall's controller"
    );
    assert!(
        on_battlefield(&engine, p0, wall_of_air()).is_some(),
        "the Wall survives the turn it arrived in"
    );
    assert!(
        !is_tapped(&engine, wall),
        "the untap step stood it back up, so nothing but the printed word can keep it out of combat"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the Wall's controller declares this combat");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 past its untap step is offered, so the combat step \
         really asked: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "\"This creature can't attack\" — Defender on an untapped creature that \
         has been under its controller's control since the turn began: {attackers:?}"
    );
}
