//! `cards/creatures/mv_5/starlit_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Starlit Angel is a `{3}{W}{W}` 3/4 Angel whose entire text is "Flying" and
/// whose file is marked `Implemented`, so the whole card is a body, a keyword
/// and a price. Five Plains pay the printed cost to the last mana — four
/// mana or six would show in what is left floating, and the offer is read
/// with the pool already full because `can_afford` reads the pool and not the
/// untapped lands. Flying is then read where it does work rather than off the
/// characteristic: a ground 1/1 across the table gets no block at all against
/// it, and the three power lands on the defending seat's life total.
#[test]
fn starlit_angel_lands_as_a_flying_three_four_that_ground_creatures_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 5])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[starlit_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana before the claim: `LegalActions` is filtered through `can_afford`,
    // which reads the pool rather than the five untapped Plains.
    let card = in_hand(&engine, p0, starlit_angel()).expect("the Angel is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains, five white — the whole printed cost"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "{{3}}{{W}}{{W}} is five mana and the pool holds exactly five: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, starlit_angel());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, starlit_angel()).is_some()
    });
    let angel = on_battlefield(&engine, p0, starlit_angel()).expect("the Angel resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed cost came out of the pool to the last mana"
    );
    assert_eq!(pt(&engine, angel), (3, 4), "the body the card prints");
    assert!(
        types(&engine, angel).contains(TypeSet::CREATURE),
        "and it arrived as the creature it prints"
    );
    assert!(
        keywords(&engine, angel).contains(KeywordSet::FLYING),
        "the one line of text the card carries"
    );

    // Flying is an evasion rule, so it is read in a block declaration: the
    // Angel has to have been under its controller's control since their turn
    // began (CR 302.6), which is one turn cycle away from the cast.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Angel's controller takes another turn"
    );
    let blockers = attack_and_collect_blocks(&mut engine, angel, p1);
    assert!(
        blockers.is_empty(),
        "CR 702.9b: a creature without flying can block only a creature \
         without flying, and the only creature across the table is a \
         ground-bound Elf: {blockers:?}"
    );

    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "the block declaration is the question under test: {:?}",
            engine.pending()
        )
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");
    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, and every life total still reads 20 there. The end step is
    // past combat damage (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "three combat damage from the unblocked 3/4"
    );
    assert!(
        on_battlefield(&engine, p0, starlit_angel()).is_some(),
        "and the Angel is still standing after the attack"
    );
}
