//! `cards/artifacts/mv_3/tooth_of_ramos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tooth of Ramos is a {3} artifact printing two mana abilities of the same
/// colour: "{T}: Add {W}" and "Sacrifice this artifact: Add {W}". The second is
/// the whole reason the card exists — it costs the permanent instead of the
/// tap — and nothing in the card file says whether the engine treats a
/// sacrifice as a price it can pay. Three Plains pay the {3} and leave the pool
/// empty, so the first white is exact and has no land behind it; the second
/// arrives off the artefact itself, which is read in its owner's graveyard
/// afterwards. Neither activation uses the stack (CR 605.3b).
#[test]
fn tooth_of_ramos_taps_and_then_sacrifices_itself_for_one_white_each() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[tooth_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tooth_of_ramos());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let tooth = on_battlefield(&engine, p0, tooth_of_ramos()).expect("the Tooth resolved");
    assert!(!is_tapped(&engine, tooth), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Plains paid the {{3}} and left nothing floating"
    );

    // Ability 0: "{T}: Add {W}." Its whole price is the tap symbol, so it is
    // offered on an empty pool and the mana lands with nothing on the stack.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tooth, 0)),
        "the printed {{T}}: Add {{W}} is offered on a board with no mana: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, tooth_of_ramos(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, tooth), "the tap was the whole price");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "{{T}}: Add {{W}} — one white, and no land on this board made it"
    );

    // Ability 1: "Sacrifice this artifact: Add {W}." Its price is not its own
    // tap, so the already-tapped Tooth is still a source — and the white it
    // makes comes out of the card rather than out of a land.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tooth, 1)),
        "a tapped Tooth still offers its sacrifice line: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, tooth_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: the sacrifice is a mana ability too, so nothing is waiting"
    );
    assert!(
        on_battlefield(&engine, p0, tooth_of_ramos()).is_none(),
        "\"Sacrifice this artifact\" takes the whole card, not merely its tap"
    );
    assert!(
        in_graveyard(&engine, p0, tooth_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "one white per printed line, both off the artefact before it is gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and nothing else is in the pool: the three Plains were spent on the cast"
    );
}
