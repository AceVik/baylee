//! `cards/creatures/mv_1/suntail_hawk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Suntail Hawk prints "{W} — 1/1 Creature — Bird" and "Flying" and nothing
/// else, so a game can hold it to exactly two things: the body that lands and
/// the evasion its one keyword buys. The Hawk that was already under p0's
/// control before the turn began is the one that attacks — the copy cast this
/// turn cannot (CR 302.6), and casting it is what shows the card arriving by
/// being played — and the block declaration is where flying is read: the
/// ground Elf across the table may not be paired with the attacker
/// (CR 509.1b), while the opposing flier may, so the offer is not simply
/// empty.
#[test]
fn suntail_hawk_flies_over_the_ground_creature_that_cannot_block_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), suntail_hawk()])
        .hand(0, &[suntail_hawk()])
        // A ground creature that must not be offered as a blocker for the
        // flier, and a flier that must: the two halves of "can this block it".
        .battlefield(1, &[llanowar_elves(), suntail_hawk()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let veteran =
        on_battlefield(&engine, p0, suntail_hawk()).expect("a Hawk was here from the start");
    cast_from_hand(&mut engine, p0, suntail_hawk());
    pass_until(&mut engine, stack_is_empty);
    let fresh = all_on_battlefield(&engine, p0, suntail_hawk())
        .into_iter()
        .find(|id| *id != veteran)
        .expect("the Hawk cast this turn is a second object, not the one already standing");
    assert_eq!(pt(&engine, fresh), (1, 1), "the body the card prints");
    assert!(
        types(&engine, fresh).contains(TypeSet::CREATURE),
        "a Bird is a creature before it is anything else"
    );
    assert!(
        keywords(&engine, fresh).contains(KeywordSet::FLYING),
        "and the one keyword the card prints reaches the permanent"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&veteran),
        "the Hawk that has been here since before the turn began may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(veteran, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the block declaration follows the attack declaration")
    };
    assert_eq!(player, p1, "the defending player is the one asked");
    let their_flier =
        on_battlefield(&engine, p1, suntail_hawk()).expect("the Hawk across the table is untapped");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("and so is the Elf");
    let may_block = |id: ObjectId| {
        blockers
            .iter()
            .any(|option| option.blocker == id && option.attackers.contains(&veteran))
    };
    assert!(
        may_block(their_flier),
        "a flier may block a flier, so the offer is not an empty one: {blockers:?}"
    );
    assert!(
        !may_block(their_elf),
        "\"flying\" is the whole evasion: a creature with neither flying nor \
         reach is not offered as this attacker's blocker (CR 509.1b): {blockers:?}"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the unblocked 1/1 flier dealt the one damage the card prints"
    );
    assert!(
        on_battlefield(&engine, p0, suntail_hawk()).is_some(),
        "and neither Hawk went anywhere: nobody blocked and nothing pointed \
         at them"
    );
}
