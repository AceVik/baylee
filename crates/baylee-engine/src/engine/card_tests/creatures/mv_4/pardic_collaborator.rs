//! `cards/creatures/mv_4/pardic_collaborator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pardic Collaborator is a {3}{R} 2/2 Human Barbarian with first strike and a
/// single activated line: "{B}: This creature gets +1/+1 until end of turn."
/// Both printed halves are read on one board, because neither is visible in the
/// other's assertion: the keyword is a projected characteristic and the pump is
/// a price, a target-free activation and a duration. The Swamp is kept back from
/// the casting so that the {B} is a real payment out of a pool only it can fill
/// (`can_afford` reads the pool, not the untapped lands), the Elf across the
/// table proves the pump is `Filter::This` and not a board sweep, and one turn
/// cycle proves "until end of turn" is a duration rather than a counter.
#[test]
fn pardic_collaborator_first_strikes_and_pumps_itself_for_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), swamp()],
        )
        .hand(0, &[pardic_collaborator()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The four Mountains pay {3}{R}; the Swamp is named as the printing kept
    // back, so it is still standing to pay for the ability below.
    tap_all_mana_but(&mut engine, p0, Some(swamp()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains and nothing else: the Swamp was kept back"
    );
    cast_with_floating(&mut engine, p0, pardic_collaborator());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let barbarian =
        on_battlefield(&engine, p0, pardic_collaborator()).expect("the Collaborator resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, barbarian), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, barbarian).contains(KeywordSet::FIRST_STRIKE),
        "the printed first strike reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the four Mountains went into the {{3}}{{R}} to the last mana"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool: with nothing floating the {B} is unpayable and the line
    // is not offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(barbarian, 0)),
        "{{B}} is not one, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // The Swamp alone, untapped and making black: one mana, and one red would
    // not have paid for the ability either.
    let land = on_battlefield(&engine, p0, swamp()).expect("the Swamp is still standing");
    assert!(!is_tapped(&engine, land), "nothing has tapped it yet");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Mountains are already down, so only the Swamp taps: one black"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(barbarian, 0)),
        "with {{B}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    // Ability 0 is the printed "{B}: This creature gets +1/+1 until end of
    // turn." It names no target, so the cost is paid with the activation.
    activate(&mut engine, p0, pardic_collaborator(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{B}} was the whole price and it came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, barbarian),
        (3, 3),
        "+1/+1 on the creature the ability is printed on"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "`Filter::This` and not a board sweep: the Elf across the table is untouched"
    );
    assert!(
        keywords(&engine, barbarian).contains(KeywordSet::FIRST_STRIKE),
        "and the pump says nothing about the keyword it already had"
    );

    // "until end of turn" is a duration and not a counter: the same permanent
    // is still standing a turn later and reads its printed body again.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, pardic_collaborator()).is_some(),
        "the creature survived the turn, so the body below is a reading of it"
    );
    assert_eq!(
        pt(&engine, barbarian),
        (2, 2),
        "the grant lasted the turn it was made in and no longer"
    );
}
