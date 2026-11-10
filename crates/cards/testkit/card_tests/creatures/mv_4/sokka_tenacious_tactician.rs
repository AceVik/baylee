//! `cards/creatures/mv_4/sokka_tenacious_tactician.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The owner's table: the house took Sokka, Tenacious Tactician with
/// Aminatou, the Fateshifter's −6, and the owner reported that Sokka still
/// triggered on the owner's spells.
///
/// Both of Sokka's "whenever you cast a noncreature spell" abilities, prowess
/// (CR 702.108a) and the Ally token, say "you", which on a permanent is its
/// controller (CR 109.5), and a triggered ability is controlled by whoever
/// controlled its source as it triggered (CR 603.3a). Once seat 1 controls
/// Sokka, seat 1's noncreature spell triggers both, under seat 1's control,
/// so the token is seat 1's. Seat 0's triggers neither: not an instant in the
/// turn the −6 resolved, and not a spell on seat 0's own next turn.
///
/// Not reproduced, here or at 0.1.0-beta.3: the engine already does this,
/// and this test keeps it doing so.
#[test]
fn sokka_taken_by_aminatou_triggers_for_his_new_controller_and_not_his_owner() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sokka_tenacious_tactician(), swamp()])
        .battlefield(1, &[aminatou()])
        .hand(0, &[dark_ritual(), mox_opal()])
        .hand(1, &[mox_opal()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let sokka = on_battlefield(&engine, p0, sokka_tenacious_tactician()).expect("Sokka is out");

    activate_aminatou_minus_six(&mut engine, p1);
    pass_until(&mut engine, stack_is_empty);
    let obj = engine.state().object(sokka).expect("Sokka");
    assert_eq!(
        (obj.controller, obj.owner),
        (p1, p0),
        "seat 1 took Sokka, and seat 0 still owns him"
    );

    // Seat 0 casts an instant in the turn Sokka changed hands.
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("seat 1 passes");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dark_ritual());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the Ritual alone: seat 0's spell triggers nothing of Sokka's"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, sokka), (3, 3), "no prowess for seat 0's spell");
    assert!(
        tokens_of(&engine, p0).is_empty() && tokens_of(&engine, p1).is_empty(),
        "and no Ally"
    );

    // Seat 1, who controls him now, casts a noncreature spell.
    cast_from_hand(&mut engine, p1, mox_opal());
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(stack.len(), 3, "the Mox, prowess and the token ability");
    assert!(
        stack
            .iter()
            .all(|id| engine.state().object(*id).map(|o| o.controller) == Some(p1)),
        "both triggers are seat 1's, who controlled Sokka as they triggered"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, sokka), (4, 4), "prowess, for seat 1's spell");
    assert_eq!(tokens_of(&engine, p1).len(), 1, "and seat 1's Ally");
    assert!(tokens_of(&engine, p0).is_empty());

    // Seat 0's own turn: Sokka is still seat 1's, and still deaf to seat 0.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().object(sokka).map(|o| o.controller),
        Some(p1),
        "the −6 lasts"
    );
    assert_eq!(
        pt(&engine, sokka),
        (3, 3),
        "the pump ended with seat 1's turn"
    );
    cast_from_hand(&mut engine, p0, mox_opal());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the Mox alone"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, sokka), (3, 3));
    assert_eq!(tokens_of(&engine, p1).len(), 1);
    assert!(tokens_of(&engine, p0).is_empty());
}

/// Sokka's other sentence under the same −6: "Other Allies you control have
/// menace and prowess" is a static ability, whose "you" is whoever controls
/// Sokka now (CR 109.5). The Ally that went to seat 1 beside him keeps
/// prowess and grows on seat 1's spell; the Ally seat 0 got from seat 1 in
/// exchange has none, and seat 0's spell grows nothing.
#[test]
fn the_prowess_sokka_lends_follows_him_to_his_new_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sokka_tenacious_tactician(), ondu_cleric(), swamp()])
        .battlefield(1, &[aminatou(), ondu_cleric()])
        .hand(0, &[dark_ritual()])
        .hand(1, &[mox_opal()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let sokka = on_battlefield(&engine, p0, sokka_tenacious_tactician()).expect("Sokka is out");
    let went = on_battlefield(&engine, p0, ondu_cleric()).expect("seat 0's Cleric");
    let came = on_battlefield(&engine, p1, ondu_cleric()).expect("seat 1's Cleric");
    assert!(keywords(&engine, went).contains(KeywordSet::PROWESS));
    assert!(!keywords(&engine, came).contains(KeywordSet::PROWESS));

    activate_aminatou_minus_six(&mut engine, p1);
    pass_until(&mut engine, stack_is_empty);
    let controller = |id| engine.state().object(id).map(|o| o.controller);
    assert_eq!(
        (controller(sokka), controller(went), controller(came)),
        (Some(p1), Some(p1), Some(p0))
    );
    assert!(
        keywords(&engine, went).contains(KeywordSet::PROWESS),
        "still another Ally Sokka's controller controls"
    );
    assert!(
        !keywords(&engine, came).contains(KeywordSet::PROWESS),
        "seat 0's new Ally is not one Sokka's controller controls"
    );

    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("seat 1 passes");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        (pt(&engine, sokka), pt(&engine, went), pt(&engine, came)),
        ((3, 3), (1, 1), (1, 1)),
        "seat 0's spell grows nobody"
    );

    cast_from_hand(&mut engine, p1, mox_opal());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        (pt(&engine, sokka), pt(&engine, went), pt(&engine, came)),
        ((4, 4), (2, 2), (1, 1)),
        "seat 1's spell grows Sokka and the Ally beside him"
    );
}
