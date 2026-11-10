//! `cards/creatures/mv_3/wall_of_blood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Blood — {2}{B} 0/2 Wall with defender and one line: "Pay 1 life:
/// This creature gets +1/+1 until end of turn."
///
/// The price names no mana at all, so the board is three Swamps the cast has
/// already spent: with the pool empty and nothing else able to make mana, the
/// only thing the ability is allowed to take is the life the two activations
/// below cost, and `(2, 4)` on a printed 0/2 is what says each payment bought
/// its own +1/+1 rather than the first one buying both. Defender is read
/// beside the numbers because it is the half of the card that must not move —
/// a pump that had quietly turned the Wall into a creature that can attack
/// would leave the P/T exactly as they are.
#[test]
fn wall_of_blood_pays_life_for_power_and_keeps_its_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[wall_of_blood()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{B} off the three Swamps, and nothing is held back: the whole price
    // read below is life, so an empty pool is what makes each claim exact.
    cast_from_hand(&mut engine, p0, wall_of_blood());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let wall = on_battlefield(&engine, p0, wall_of_blood()).expect("the Wall resolved");
    assert_eq!(pt(&engine, wall), (0, 2), "the body the card prints");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "the printed defender, before anything is paid into it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Swamps paid for the Wall, so no mana is left for the pump to cost"
    );

    // Ability 0 is the card's only line, and `can_afford` has nothing but the
    // life total to read here — an empty pool does not withhold it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(wall, 0)),
        "with an empty pool the pump is still offered, because its price is \
         life and not mana: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, wall_of_blood(), 0);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the life is the last step of the activation (CR 601.2h)"
    );
    assert!(
        !stack_is_empty(&engine),
        "paying life is no mana ability, so the pump is on the stack"
    );
    assert_eq!(
        pt(&engine, wall),
        (0, 2),
        "and nothing has happened to the body yet"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, wall), (1, 3), "+1/+1 until end of turn, once");

    // A second activation is a second life: the pump is priced per point and
    // not a bonus the Wall gets for having been asked once.
    activate(&mut engine, p0, wall_of_blood(), 0);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, wall), (2, 4), "a second +1/+1");
    assert_eq!(
        engine.state().players[0].life,
        18,
        "and a second life paid for it"
    );
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "still a Wall: nothing about the pump touches the keyword"
    );
}
