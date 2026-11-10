//! `cards/artifacts/mv_4/ur_golem_s_eye.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ur-Golem's Eye prints one line — "{T}: Add {C}{C}" — on a {4} artifact that
/// enters untapped, so both halves of the card are read inside one main phase:
/// four Forests pay the {4} down to an empty pool, and the eye then pays its
/// own tap symbol for two mana that no land on this board could have produced.
/// Two *colorless* is the whole printed quantity, and the green the Forests
/// make is read at zero afterwards — a permanent that had added {G}{G} would
/// fill the same pool to the same total and the same colour reading twice.
#[test]
fn ur_golems_eye_taps_for_two_colorless_off_an_empty_pool() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[ur_golem_s_eye()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests are exactly {4} — the artifact's whole cost — so the pool
    // is empty the moment it lands and nothing floating could be mistaken for
    // the mana its own ability makes below.
    cast_from_hand(&mut engine, p0, ur_golem_s_eye());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let eye = on_battlefield(&engine, p0, ur_golem_s_eye()).expect("the Eye resolved");
    assert!(
        types(&engine, eye).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, eye), "and an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Forests paid the {{4}} to the last mana"
    );

    // Ability 0 is the printed "{T}: Add {C}{C}", whose whole price is its own
    // tap, so it is offered on an empty pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(eye, 0)),
        "the one line the card prints costs its own {{T}} and nothing else: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, ur_golem_s_eye(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is a fixed colourless and not \"any color\", so nothing is \
         asked on the way: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "\"{{T}}: Add {{C}}{{C}}\" — two, off one tap"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forests make green and are spent by now, so the two mana on the \
         pool cannot be theirs"
    );
    assert_eq!(pool.total(), 2, "two mana, and nothing else came with them");
    assert!(is_tapped(&engine, eye), "the Eye paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
}
