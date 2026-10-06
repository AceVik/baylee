//! `cards/creatures/mv_7/angel_of_retribution.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Angel of Retribution is `{6}{W}` for a 5/5 Angel whose entire printed text
/// is two keywords: flying and first strike. The scenario plays the card
/// instead of reading its file — the pool is filled before anything is
/// claimed about affordability, because `can_afford` reads the pool and not
/// the untapped lands, and the seven mana are then spent to the last drop, so
/// the permanent on the battlefield is one a real cast put there. Both
/// keywords are read off the object the layer system projects, the only
/// reading that can tell a printed keyword from one a neighbouring effect
/// hands out; a second, one-Plains-short board is the control that says the
/// cost really is `{6}{W}` and not `{5}{W}`.
#[test]
fn angel_of_retribution_arrives_for_six_and_a_white_as_the_five_five_flier_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4111, plains())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[angel_of_retribution()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `LegalActions::castable` is filtered through `can_afford`, and that
    // reads the pool rather than the seven untapped Plains: with nothing
    // floating, the Angel is not among the castable cards at all.
    let card = in_hand(&engine, p0, angel_of_retribution()).expect("the Angel is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{6}}{{W}}, and `can_afford` reads the pool \
         rather than the untapped lands: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Plains and nothing else on the board: seven white"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with seven white floating the printed cost is payable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, angel_of_retribution());
    pass_until(&mut engine, stack_is_empty);
    let angel = on_battlefield(&engine, p0, angel_of_retribution())
        .expect("the Angel resolved onto the battlefield under its caster");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{6}}{{W}} came out of the pool to the last mana"
    );
    assert_eq!(
        pt(&engine, angel),
        (5, 5),
        "the body the card prints, read after the layers have run"
    );
    let printed = keywords(&engine, angel);
    assert!(
        printed.contains(KeywordSet::FLYING),
        "flying is on the permanent and not merely in the card file"
    );
    assert!(
        printed.contains(KeywordSet::FIRST_STRIKE),
        "and first strike beside it, from the same printed line"
    );

    // The cost is `{6}{W}` and not `{5}{W}`: the same board one Plains short
    // taps to six, which is one mana less than the printed price, and the
    // Angel is not offered as castable.
    let mut short = Duel::new(4112, plains())
        .battlefield(0, &[plains(); 6])
        .hand(0, &[angel_of_retribution()])
        .start();
    keep_mulligans(&mut short);
    reach_main_phase(&mut short, p0);
    tap_all_mana(&mut short, p0);
    let short_card = in_hand(&short, p0, angel_of_retribution()).expect("the Angel is in hand");
    assert_eq!(
        short.state().players[0].mana_pool.total(),
        6,
        "six Plains, six white"
    );
    let Pending::Priority { legal, .. } = short.pending().clone() else {
        panic!("expected priority, got {:?}", short.pending())
    };
    assert!(
        !legal.castable.contains(&short_card),
        "six is not the seven mana {{6}}{{W}} asks for: {:?}",
        legal.castable
    );
}
