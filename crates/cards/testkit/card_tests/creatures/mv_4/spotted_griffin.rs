//! `cards/creatures/mv_4/spotted_griffin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "4916773d-5ccb-48ff-8aa3-09771ae88e81"

/// Spotted Griffin prints a `{3}{W}` cost for a 2/3 Griffin with flying and
/// nothing else, so all three of those claims have to be read off the
/// permanent the card became rather than off the card file — a `(2, 3)` body
/// and a `FLYING` keyword are what the layer system projects, and neither is
/// visible on the card that is still in hand. Four Plains pay for it, and the
/// offer is claimed with the mana already floating, because
/// `LegalActions::castable` is filtered through `can_afford`, which reads the
/// pool and not the untapped lands: an empty pool is the reason the Griffin
/// is uncastable above, and nothing else on this board could refuse it.
#[test]
fn spotted_griffin_lands_as_a_two_three_flier_for_its_printed_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[spotted_griffin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four untapped Plains pay no {3}{W} until the pool holds it: the price is
    // the only thing standing between the card and the battlefield, since a
    // 2/3 flier for four asks for no target and no mode.
    let card = in_hand(&engine, p0, spotted_griffin()).expect("the Griffin is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{W}}, so the Griffin is not yet castable: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains, four white, and the Forests beside them make nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with four floating the printed cost is affordable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, spotted_griffin());
    pass_until(&mut engine, stack_is_empty);

    let griffin = on_battlefield(&engine, p0, spotted_griffin()).expect("the Griffin resolved");
    assert_eq!(
        pt(&engine, griffin),
        (2, 3),
        "the printed 2/3 body, read after the layer system has run"
    );
    assert!(
        keywords(&engine, griffin).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent it became"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{W}} came out of the pool rather than sitting beside it"
    );
    assert!(
        in_graveyard(&engine, p0, spotted_griffin()).is_none(),
        "the spell resolved onto the battlefield, so its card is in no graveyard"
    );
}
