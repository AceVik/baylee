//! `cards/creatures/mv_1/goblin_balloon_brigade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Balloon Brigade prints one line and nothing else — "{R}: This
/// creature gains flying until end of turn" — so the card is only itself when
/// both halves of that sentence are played: the second red is a real price out
/// of a pool the two Mountains actually paid into, and the keyword it buys
/// outlives the ability's resolution without outliving the turn. Both are read
/// off the same 1/1 Goblin: no flying before anybody pays, flying after the
/// pump resolves with its body still a printed 1/1 (+0/+0), and the keyword
/// gone once the opponent is the active player — which a permanent grant, and
/// a grant that never landed at all, would each fail from opposite sides.
#[test]
fn goblin_balloon_brigade_buys_flying_for_one_red_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[goblin_balloon_brigade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Both Mountains pay the {R} and leave the ability's own {R} beside it in
    // the pool. One main phase, so CR 500.5 does not empty it in between.
    cast_from_hand(&mut engine, p0, goblin_balloon_brigade());
    pass_until(&mut engine, stack_is_empty);
    let goblin =
        on_battlefield(&engine, p0, goblin_balloon_brigade()).expect("the Brigade resolved");
    assert_eq!(pt(&engine, goblin), (1, 1), "a printed 1/1");
    assert!(
        !keywords(&engine, goblin).contains(KeywordSet::FLYING),
        "and a ground creature until somebody pays for the balloon"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one Mountain bought the Goblin and the other is exactly the {{R}} \
         the ability charges"
    );

    activate(&mut engine, p0, goblin_balloon_brigade(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it uses the stack"
    );
    assert!(
        !keywords(&engine, goblin).contains(KeywordSet::FLYING),
        "and the keyword has not arrived before the ability resolves"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, goblin).contains(KeywordSet::FLYING),
        "{{R}}: this creature gains flying until end of turn"
    );
    assert_eq!(
        pt(&engine, goblin),
        (1, 1),
        "the pump is +0/+0: a flying 1/1, not a bigger one"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{R}} was the price, so nothing is left floating"
    );

    // The duration half: nothing kills the Goblin, nothing else touches it,
    // and the only thing that changed is whose turn it is.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, goblin_balloon_brigade()).is_some(),
        "the Goblin is still on the battlefield to be read"
    );
    assert!(
        !keywords(&engine, goblin).contains(KeywordSet::FLYING),
        "\"until end of turn\": the balloon came down with the turn it was \
         paid for, so the grant is not a permanent one"
    );
    assert_eq!(pt(&engine, goblin), (1, 1), "and the body never changed");
}
