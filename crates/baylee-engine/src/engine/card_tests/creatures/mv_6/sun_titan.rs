//! `cards/creatures/mv_6/sun_titan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sun Titan — {4}{W}{W}, 6/6 vigilance — "whenever this creature enters or
/// attacks, you may return target permanent card with mana value 3 or less
/// from your graveyard to the battlefield."
///
/// The clause that can be read wrong is the price, so the graveyard is built
/// to say it: Llanowar Elves at 1, Rib Cage Spider at 3 — the boundary, and
/// "or less" is what puts it on the menu — and Wild Elephant at 4, which is
/// the one that must not be there. Three cards and two answers, so an offer
/// that listed everything and an offer that listed nothing are both visible.
#[test]
fn sun_titan_returns_a_permanent_of_three_or_less_and_is_not_offered_the_four() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                rib_cage_spider(),
                wild_elephant(),
            ],
        )
        .hand(0, &[sun_titan()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");
    let elephant = on_battlefield(&engine, p0, wild_elephant()).expect("the Elephant is seated");
    bury(&mut engine, &[elves, spider, elephant]);
    assert!(
        in_graveyard(&engine, p0, wild_elephant()).is_some(),
        "the Elephant is in the graveyard, so being left off the offer below \
         is the mana value and not the zone"
    );

    cast_from_hand(&mut engine, p0, sun_titan());
    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&elves) && options.contains(&spider),
        "mana value 1 and mana value 3 are both \"3 or less\": {options:?}"
    );
    assert!(
        !options.contains(&elephant),
        "and mana value 4 is not: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![spider],
                players: vec![],
            },
        )
        .expect("the Spider was on the menu");
    pass_until(&mut engine, stack_is_empty);

    let titan = on_battlefield(&engine, p0, sun_titan()).expect("the Titan resolved");
    assert_eq!(pt(&engine, titan), (6, 6), "the body the card prints");
    let back = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider came back");
    assert_eq!(pt(&engine, back), (1, 4), "as the creature it is");
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "\"target permanent card\" is one card and not the whole graveyard"
    );
}

/// Sun Titan: "Whenever this creature enters or attacks, you may return
/// target permanent card with mana value 3 or less from your graveyard."
/// This is the attack half; the Titan was never cast, so only an attack
/// can have returned the Elves.
#[test]
fn sun_titan_returns_a_small_permanent_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2203, forest())
        .battlefield(0, &[sun_titan(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let titan = on_battlefield(&engine, p0, sun_titan()).expect("titan");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves");
    bury(&mut engine, &[elves]);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());

    unf_attack(&mut engine, p0, &[titan]);
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}
