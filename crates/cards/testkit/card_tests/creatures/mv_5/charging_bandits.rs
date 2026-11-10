//! `cards/creatures/mv_5/charging_bandits.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Charging Bandits — {4}{B}, a 3/3 Human Rogue whose entire printed text is
/// "Whenever this creature attacks, it gets +2/+0 until end of turn."
///
/// The trigger only means something measured against both of its alternatives,
/// so the board carries a Llanowar Elves that is *not* the creature being
/// pumped, and the scenario is walked far enough to read both ends of the
/// duration: the Bandits attacks as a 5/3 and deals five rather than the three
/// it prints, and is a printed 3/3 again on its controller's next turn. That
/// it is cast for real is what puts summoning sickness in the way, so the
/// attack happens a turn after the cast.
#[test]
fn charging_bandits_pumps_itself_only_when_it_attacks_and_only_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[charging_bandits()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Swamps pay {{4}}{{B}}, and the Elf is named as the printing kept
    // back: it is the control this test reads back after the attack, and a
    // mana creature that had paid for the cast would be tapped for it.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Swamps, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, charging_bandits());
    pass_until(&mut engine, stack_is_empty);
    let bandits = on_battlefield(&engine, p0, charging_bandits()).expect("the Bandits resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, bandits), (3, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{B}} came out of the pool"
    );

    // CR 302.6: the Bandits has not been under its controller's control since
    // its turn began, so its first chance to attack is the next one — and the
    // turn in between is walked rather than skipped.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, bandits),
        (3, 3),
        "it has not attacked yet, so nothing has pumped it"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the Bandits' controller is the one declaring");
    assert!(
        attackers.contains(&bandits),
        "an untapped 3/3 that has been under its controller since the turn \
         began may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bandits, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // The trigger goes on the stack behind the declaration, so the board is
    // read once it has resolved.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, bandits),
        (5, 3),
        "\"it gets +2/+0\": the 3/3 is a 5/3 for the rest of the turn"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the trigger names the attacking permanent and no other creature, so \
         the Elf beside it is untouched"
    );

    // Five, not three. The damage step is the one place the pump has to be
    // live for the printed sentence to have meant anything.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        15,
        "a 3/3 that dealt three damage would leave the opponent at 17"
    );

    // "until end of turn": one turn later the body is the printed 3/3 again.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, bandits),
        (3, 3),
        "the pump lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, charging_bandits()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
