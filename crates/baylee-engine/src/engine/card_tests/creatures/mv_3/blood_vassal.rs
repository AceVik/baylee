//! `cards/creatures/mv_3/blood_vassal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blood Vassal is a {2}{B} 2/2 Thrull whose whole printed text is one mana
/// ability: "Sacrifice this creature: Add {B}{B}." Both halves of that price
/// are the engine's answer rather than the card's, so one activation is read
/// against a board that can account for every drop of mana: three Swamps pay
/// for the creature and are left tapped, which is what makes the two black
/// that appear afterwards impossible to source from anywhere else.
///
/// The price is a sacrifice and not a tap, so `tap_all_mana` never presses it
/// (#159) and the pool is empty when the ability is claimed —
/// `legal.abilities` is filtered through `can_afford`, which reads the pool.
/// The empty stack afterwards is CR 605.3b, and the creature in its owner's
/// graveyard is the other half of the cost actually paid.
#[test]
fn blood_vassal_sacrifices_itself_for_two_black_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[blood_vassal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, blood_vassal());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let vassal = on_battlefield(&engine, p0, blood_vassal()).expect("the Vassal resolved");
    assert_eq!(pt(&engine, vassal), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Swamps paid the {{2}}{{B}} and nothing is left floating"
    );
    assert!(
        all_on_battlefield(&engine, p0, swamp())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "and every Swamp is spent, so no black source is left standing on the board"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vassal, 0)),
        "the one line the card prints is offered with an empty pool, because \
         its price is the creature and not its own tap: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, blood_vassal(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        2,
        "Add {{B}}{{B}} — two black, and not one per activation"
    );
    assert_eq!(pool.total(), 2, "two black and nothing else came with them");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        on_battlefield(&engine, p0, blood_vassal()).is_none(),
        "sacrificing the creature is the whole of the price"
    );
    assert!(
        in_graveyard(&engine, p0, blood_vassal()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
