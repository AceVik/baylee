//! `cards/creatures/mv_3/ravenous_skirge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ravenous Skirge — {2}{B} Creature — Phyrexian Imp 1/1: "Flying" and
/// "Whenever this creature attacks, it gets +2/+0 until end of turn."
///
/// The pump is aimed at `Filter::This`, so the board carries a second creature
/// under the same seat and one across the table: both must still read their
/// printed 1/1 once the attack trigger has resolved, which a static that had
/// widened to "creatures you control" would not. `(3, 1)` is the only body on
/// the attacker that reads the printed +2/+0 — `(3, 3)` would be a toughness
/// nobody granted — and the life total the turn ends on is where the two
/// granted power has to have counted in combat, on a 1/1 that would otherwise
/// deal one.
#[test]
fn ravenous_skirge_flies_and_pumps_itself_only_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[ravenous_skirge()])
        // A creature across the table, so "it gets" has something it must not
        // reach, and so the attacker's flying has a board to fly over.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The {2}{B} comes out of the three Swamps, and the Elf is named as the
    // printing kept back: `tap_all_mana` would have spent its own {T} and
    // taken it out of the board every assertion below reads.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps, and the Elf paid nothing"
    );
    cast_with_floating(&mut engine, p0, ravenous_skirge());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });

    let skirge = on_battlefield(&engine, p0, ravenous_skirge()).expect("the Skirge resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, skirge),
        (1, 1),
        "a printed 1/1 before it attacks"
    );
    assert!(
        keywords(&engine, skirge).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );

    // The Skirge arrived this turn, so CR 302.6 keeps it out of this
    // combat: the attack trigger is read one turn cycle later.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. }) && e.state().turn.active == p0
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&skirge),
        "an untapped 1/1 may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(skirge, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The pump is a trigger and not the declaration itself, so the walk passes
    // priority until it has actually resolved.
    pass_until(&mut engine, |e| pt(e, skirge) == (3, 1));
    assert_eq!(
        pt(&engine, skirge),
        (3, 1),
        "\"Whenever this creature attacks, it gets +2/+0 until end of turn\""
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the trigger is aimed at the attacking creature and no other"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "nor across the table");

    // And the pump has to be worth something: nothing across the table can
    // block a flier, so three damage is where the +2/+0 is read off a life
    // total rather than off the projection alone.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "three combat damage — one printed power plus the two the trigger granted"
    );
}
