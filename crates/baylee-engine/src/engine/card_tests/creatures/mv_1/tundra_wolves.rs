//! `cards/creatures/mv_1/tundra_wolves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tundra Wolves is a `{W}` 1/1 Wolf whose whole printed text is "First
/// strike", and a keyword is only worth anything in the damage step: a 1/1
/// that blocks a 1/1 would trade, while a first striker kills the attacker in
/// the first-strike damage step and never takes the return damage. So the
/// scenario is the trade that does *not* happen — the Wolf blocks a Llanowar
/// Elves, the Elves die, the Wolf stands — with the Elves as the control: on
/// the same board without the keyword both cards would be in a graveyard.
/// The Wolf was cast this turn and so may not attack (CR 302.6), which is why
/// it is the blocking half of the pair that carries the claim.
#[test]
fn tundra_wolves_kills_what_it_blocks_before_it_can_strike_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[tundra_wolves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, tundra_wolves());
    pass_until(&mut engine, stack_is_empty);
    let wolf = on_battlefield(&engine, p0, tundra_wolves()).expect("the Wolf resolved");
    let attacker = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, wolf), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, wolf).contains(KeywordSet::FIRST_STRIKE),
        "the card's whole text reaches the permanent"
    );
    assert!(
        !keywords(&engine, attacker).contains(KeywordSet::FIRST_STRIKE),
        "and the creature it is about to block has none, or the two would \
         strike together and this scenario would prove nothing"
    );

    // The Wolf was cast this turn, so it may not attack — but blocking is
    // neither an attack nor a `{T}` cost, and the Elves have been standing
    // since the game started.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let life_before = engine.state().players[0].life;
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .expect("an untapped 1/1 with no text of its own may attack");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p0),
    );
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == wolf && b.attackers.contains(&attacker)),
        "the Wolf is offered as a blocker for the Elves: {blockers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wolf, attacker)],
            },
        )
        .expect("the pairing the offer named is a legal block");

    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
            && matches!(e.pending(), Pending::Priority { .. })
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "1 damage from a first striker is lethal to a 1/1"
    );
    assert!(
        on_battlefield(&engine, p0, tundra_wolves()).is_some(),
        "and the Wolf never takes the Elves' damage back: without first \
         strike this is a trade and both cards are in a graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, tundra_wolves()).is_none(),
        "which is the half that first strike buys"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the attacker was blocked, so nothing reached the player"
    );
}
