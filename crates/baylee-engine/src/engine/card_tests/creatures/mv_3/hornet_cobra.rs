//! `cards/creatures/mv_3/hornet_cobra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hornet Cobra is a printed 2/1 Snake whose entire text is "First strike",
/// so the only thing worth playing is a combat it survives by killing its
/// blocker before the blocker can strike back. It is cast off three Forests
/// on p0's own turn and attacks one turn later, because a creature that
/// arrived this turn may not attack (CR 302.6). The 1/1 Llanowar Elves that
/// blocks it deals the one damage that is lethal to a 1-toughness attacker,
/// so the Cobra standing at the end of the turn is the first-strike damage
/// step and nothing else.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn hornet_cobra_kills_its_blocker_before_the_blocker_can_strike_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(821, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[hornet_cobra()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches their own main"
    );

    // {1}{G}{G} off the three Forests, so the 2/1 on the table arrived the
    // way the card arrives rather than by being seated there.
    cast_from_hand(&mut engine, p0, hornet_cobra());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cobra = on_battlefield(&engine, p0, hornet_cobra()).expect("the Cobra resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is seated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid exactly {{1}}{{G}}{{G}}"
    );
    assert_eq!(pt(&engine, cobra), (2, 1), "the printed 2/1 body");
    assert!(
        keywords(&engine, cobra).contains(KeywordSet::FIRST_STRIKE),
        "and the whole of the card's text, projected onto the permanent"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the blocker is a printed 1/1: its one damage is lethal to a 2/1, \
         which is exactly what the first strike has to avoid"
    );

    // A turn later, because a creature that entered this turn may not attack
    // (CR 302.6) — the untap step is what makes the attack below legal.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, cobra),
        "the untap step stood the Cobra back up"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&cobra),
        "an untapped, unsick 2/1 may attack: {attackers:?}"
    );
    assert!(
        defenders
            .iter()
            .any(|d| matches!(d, Defender::Player(p) if *p == p1)),
        "and the seat across the table is what it may be aimed at: {defenders:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cobra, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player,
        attacker,
        blockers,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    assert_eq!(player, p1, "the defending seat declares the blockers");
    assert_eq!(attacker, p0, "and it is p0's attack being blocked");
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&cobra)),
        "the Elf is offered against the Cobra: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, cobra)],
            },
        )
        .unwrap();

    // Past the combat damage step to the end step: the stack is empty the
    // moment blockers are declared, so `stack_is_empty` would stop the walk
    // before any damage was dealt at all.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two first-strike damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p0, hornet_cobra()).is_some(),
        "and the Cobra is still standing, which is the whole card: without \
         first strike the Elf's one damage would have landed in the same step \
         and killed it too"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the two damage went into the blocker, so nothing reached the player"
    );
}
