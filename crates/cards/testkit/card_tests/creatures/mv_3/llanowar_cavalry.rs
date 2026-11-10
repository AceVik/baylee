//! `cards/creatures/mv_3/llanowar_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Llanowar Cavalry is a printed `{2}{G}` 1/4 with one activated line —
/// "`{W}`: This creature gains vigilance until end of turn." — and nothing
/// else, so the whole card is the difference between a body and a body that
/// can attack and still block. The board separates the two halves of the
/// price: three Forests pay the `{2}{G}` while the Plains is named as the
/// thing kept back, because it is the only source on the table that can pay
/// the `{W}`, and the offer for the ability is read twice — absent on an
/// empty pool and present once white is floating — which is the reading
/// `can_afford` actually does (it reads the pool, never the untapped lands).
/// The keyword is read off the layer projection afterwards, where a granted
/// vigilance is the only kind that can be seen.
#[test]
fn llanowar_cavalry_spends_a_white_for_vigilance_it_never_printed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), plains()])
        .hand(0, &[llanowar_cavalry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{G} off the three Forests, with the Plains named as the source kept
    // back: it is the only one that can pay the {W} the ability charges, and
    // a Plains tapped for the creature would leave the second half of the
    // card unpayable for the rest of the turn.
    tap_all_mana_but(&mut engine, p0, Some(plains()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests tapped, and the Plains still standing for the {{W}}"
    );
    cast_with_floating(&mut engine, p0, llanowar_cavalry());
    pass_until(&mut engine, stack_is_empty);

    let cavalry = on_battlefield(&engine, p0, llanowar_cavalry()).expect("the Cavalry resolved");
    assert_eq!(pt(&engine, cavalry), (1, 4), "the body the card prints");
    assert!(
        !keywords(&engine, cavalry).contains(KeywordSet::VIGILANCE),
        "and no vigilance until something is spent on it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast's three mana left nothing floating"
    );

    // An ability is offered or not by what the pool can pay, not by what is
    // untapped: an empty pool pays no {W}, so the line is not there yet.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cavalry, 0)),
        "an unpayable {{W}} is how the ability is absent rather than refused: {:?}",
        legal.abilities
    );

    let land = on_battlefield(&engine, p0, plains()).expect("the Plains is still standing");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("an untapped Plains pays its own {{T}}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one white, off the one source that makes it"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cavalry, 0)),
        "with the {{W}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, llanowar_cavalry(), 0);
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );
    assert!(
        !keywords(&engine, cavalry).contains(KeywordSet::VIGILANCE),
        "and nothing has happened yet — the keyword waits for the resolution"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} came out of the pool to pay for it"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, cavalry).contains(KeywordSet::VIGILANCE),
        "\"This creature gains vigilance until end of turn\""
    );
    assert_eq!(
        pt(&engine, cavalry),
        (1, 4),
        "and a keyword is all it bought: the body is untouched"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_cavalry()).is_some(),
        "the creature is still on the battlefield, untapped and now able to block"
    );
}
