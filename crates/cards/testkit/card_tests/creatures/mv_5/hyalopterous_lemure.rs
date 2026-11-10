//! `cards/creatures/mv_5/hyalopterous_lemure.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hyalopterous Lemure is a {4}{B} 4/3 Spirit printing one line: "{0}: This
/// creature gets -1/-0 and gains flying until end of turn." The card *is*
/// that trade, so the scenario pays the price twice and reads both halves each
/// time: the printed 4/3 becomes 3/3 and then 2/3 while the toughness never
/// moves, and flying arrives with the first activation. A second activation is
/// what tells "gets -1/-0" from a one-shot "becomes a 3/3" — one caps, the
/// other keeps subtracting. The walk into the opponent's turn is the printed
/// duration, because a grant that never expired would satisfy every assertion
/// above it.
#[test]
fn hyalopterous_lemure_trades_a_power_for_flying_and_gives_it_back_at_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hyalopterous_lemure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4}{B} off five Swamps: a pool survives until the step ends (CR 500.5)
    // and the whole scenario lives inside this one main phase.
    cast_from_hand(&mut engine, p0, hyalopterous_lemure());
    pass_until(&mut engine, stack_is_empty);
    let lemure = on_battlefield(&engine, p0, hyalopterous_lemure()).expect("the Lemure resolved");
    assert_eq!(pt(&engine, lemure), (4, 3), "the printed 4/3 body");
    assert!(
        !keywords(&engine, lemure).contains(KeywordSet::FLYING),
        "nothing printed on the card grants evasion, so it starts on the ground"
    );

    // The price is `{0}`, so an empty pool withholds nothing: there is no mana
    // in `can_afford` to be short of, and the offer is read on the board the
    // cast itself left behind. The Swamps are all spent, which is what makes
    // the pool reading below exact.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Swamps paid the {{4}}{{B}} to the last mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lemure, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, hyalopterous_lemure(), 0);
    assert!(
        !stack_is_empty(&engine),
        "the pump is no mana ability, so the ability goes on the stack"
    );
    assert_eq!(
        pt(&engine, lemure),
        (4, 3),
        "and nothing has happened while it is still waiting there"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, lemure),
        (3, 3),
        "-1/-0 takes one power and no toughness"
    );
    assert!(
        keywords(&engine, lemure).contains(KeywordSet::FLYING),
        "and the same activation is what grants the flying"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{0}} is a real price — nothing — so the pool is exactly where it was"
    );

    // The second activation is what tells a subtraction from a transformation:
    // a creature that "became 3/3" would still read 3/3 here.
    activate(&mut engine, p0, hyalopterous_lemure(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, lemure),
        (2, 3),
        "a second {{0}} is a second -1/-0 on the very same creature"
    );
    assert!(
        !is_tapped(&engine, lemure),
        "the price names no {{T}}, so the Lemure is still standing to block"
    );

    // "until end of turn" is the half nothing above can see: the effect ends in
    // the cleanup of the turn it was made in, so the same creature on the
    // opponent's turn is the 4/3 it was printed as.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, lemure),
        (4, 3),
        "the turn ended and both halves of the pump with it"
    );
    assert!(
        !keywords(&engine, lemure).contains(KeywordSet::FLYING),
        "and the flying left with the same duration, not with the creature"
    );
    assert!(
        on_battlefield(&engine, p0, hyalopterous_lemure()).is_some(),
        "the Lemure is still standing, so the pump left rather than the creature"
    );
}
