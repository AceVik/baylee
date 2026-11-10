//! `cards/creatures/mv_2/royal_falcon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Royal Falcon is `{1}{W}` for a 1/1 Bird whose whole printed text is
/// "Flying". A keyword on a vanilla body can only be read off the board it
/// stands on, so the card is cast for real and then sent into combat: a
/// grounded Elf beside it is offered as a blocker of the ground attacker in
/// the very same declaration and refused as a blocker of the Bird. `(1, 1)`
/// plus flying, and the {1}{W} actually paid out of the pool, is the entire
/// card — both halves are asserted rather than one.
#[test]
fn royal_falcon_lands_as_a_one_one_flier_a_grounded_elf_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[royal_falcon()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf beside the Bird is kept untapped: it is this test's *ground*
    // attacker, and a creature tapped for its own mana is no longer one. The
    // two Plains are exactly the printed `{1}{W}` and nothing else.
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains, and not the Elf beside them"
    );
    cast_with_floating(&mut engine, p0, royal_falcon());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let falcon = on_battlefield(&engine, p0, royal_falcon()).expect("the Falcon resolved");
    assert_eq!(pt(&engine, falcon), (1, 1), "a printed 1/1 Bird");
    assert!(
        types(&engine, falcon).contains(TypeSet::CREATURE),
        "and a creature"
    );
    assert!(
        keywords(&engine, falcon).contains(KeywordSet::FLYING),
        "the one word the card prints reaches the permanent"
    );
    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::FLYING),
        "the Elf beside it is grounded, which is the other half of that reading"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{W}} came out of the pool"
    );

    // A turn cycle: the Bird was cast this turn and has summoning sickness
    // (CR 302.6), so the combat its keyword is read in is the next one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&falcon) && attackers.contains(&my_elf),
        "both untapped creatures under p0 may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (falcon, Defender::Player(p1)),
                    (my_elf, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    // The blocking question is read and not merely tolerated: the ground
    // Elf is the control that keeps the second assertion from being an empty
    // board's.
    // CR 508.2 hands priority round once the attack is declared, so the
    // blocker menu is not the next thing the engine says.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "a legal block exists, so the defending seat is asked: {:?}",
            engine.pending()
        )
    };
    let paired = |blocker: ObjectId, attacker: ObjectId| {
        blockers
            .iter()
            .any(|o| o.blocker == blocker && o.attackers.contains(&attacker))
    };
    assert!(
        paired(their_elf, my_elf),
        "two grounded 1/1s: the Elf across the table may block the one I \
         attacked with: {blockers:?}"
    );
    assert!(
        !paired(their_elf, falcon),
        "\"Flying\" — a creature with no flying may not block it: {blockers:?}"
    );
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Nothing blocked, so the flier's point of damage lands on the same board
    // as the Elf's: one combat step, two life.
    pass_until(&mut engine, |e| e.state().players[1].life < 20);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the Bird connected for 1 and the ground attacker for 1"
    );
}
