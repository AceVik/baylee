//! `cards/artifacts/mv_3/seashell_cameo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seashell Cameo — {3} artifact: "{T}: Add {W} or {U}."
///
/// The one activation has to both ask and pay, so the board is built so that
/// neither can be borrowed: four Forests produce the {3} and nothing but {G},
/// and the single blue in the pool afterwards therefore came off the Cameo's
/// own tap. The two colours it offers are asserted as the enumeration itself —
/// a card that had lost one half of "or" would still fill a pool of one mana —
/// and the tapped artifact at the end says the {T} was the whole price of that
/// offer rather than a label on a free ability.
#[test]
fn seashell_cameo_taps_for_white_or_blue_whichever_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[seashell_cameo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests pay the {3} and leave one green behind; the cameo is in
    // hand while they are tapped, so its own tap is still there to spend.
    cast_from_hand(&mut engine, p0, seashell_cameo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cameo = on_battlefield(&engine, p0, seashell_cameo()).expect("the Cameo resolved");
    assert!(
        types(&engine, cameo).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact the card prints"
    );
    assert!(!is_tapped(&engine, cameo), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four Forests paid the {{3}} and exactly one green is left"
    );

    // Ability 0 is the printed "{T}: Add {W} or {U}". A mana ability a card
    // prints has an index to name, so it is an ordinary entry in `abilities`
    // and never the CR 305.6 shortcut a basic land uses.
    activate(&mut engine, p0, seashell_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "the two colours the card prints, and no third"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and not the other half of the pair"
    );
    assert_eq!(
        pool.total(),
        2,
        "the Forest's green beside the one mana the Cameo made"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");

    // The whole price was that tap, so the line is no longer one the seat may
    // take — read off the offer, which is where an unpayable cost goes.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cameo, 0)),
        "a tapped Cameo has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
