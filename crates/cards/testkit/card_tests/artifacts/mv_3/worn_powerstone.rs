//! `cards/artifacts/mv_3/worn_powerstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Worn Powerstone — {3} artifact: "This artifact enters tapped" and
/// "{T}: Add {C}{C}".
///
/// The two printed lines are read in one game because each is the other's
/// control. The entry is asserted on an **empty** pool — three Forests paid
/// the {3} and the stone made nothing on the way in, so nothing on this
/// board could have supplied the tap the card prints — and the mana ability
/// is only reachable a turn later, once the untap step has stood the stone
/// back up, where `{T}` is the whole price and exactly two colourless arrive.
#[test]
fn worn_powerstone_enters_tapped_and_taps_for_two_colorless_on_a_later_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[worn_powerstone()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The three Forests are the whole cost, so the pool is empty the moment
    // the artifact resolves: nothing floats, and an artifact that was never
    // on the battlefield while mana was being made cannot have tapped itself.
    cast_from_hand(&mut engine, p0, worn_powerstone());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, worn_powerstone()).is_some()
    });
    let stone = on_battlefield(&engine, p0, worn_powerstone()).expect("the stone resolved");
    assert!(is_tapped(&engine, stone), "\"This artifact enters tapped\"");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} was spent and the stone made nothing on the way in"
    );
    assert!(
        types(&engine, stone).contains(TypeSet::ARTIFACT),
        "and what arrived is the artifact it prints"
    );

    // `{T}` has no price to pay while the stone is lying tapped, so reading
    // the second printed line needs a turn: the untap step is what turns it
    // into an ability the seat is offered at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, stone),
        "the untap step stood the stone back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    activate(&mut engine, p0, worn_powerstone(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "\"{{T}}: Add {{C}}{{C}}\" — two, off one tap"
    );
    assert_eq!(pool.total(), 2, "and nothing else came with them");
    assert!(is_tapped(&engine, stone), "the stone paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
}
