//! `cards/creatures/mv_1/serra_zealot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Serra Zealot is a {W} 1/1 Human Soldier whose entire printed text is
/// "First strike", so the card only exists in the combat damage step: a 1/1
/// that blocks another 1/1 has to kill it in the first strike damage step
/// (CR 510.4) and still be standing when the regular one would have begun.
/// A plain 1/1 trades there, so the two assertions below are the whole claim
/// together — the attacker died, which can only be the Zealot's one damage,
/// and the Zealot lived, which can only mean the attacker's damage never
/// happened. Reading the keyword off the permanent would be satisfied by a
/// static that grants it and nothing that acts on it.
#[test]
fn serra_zealot_first_strikes_the_one_one_it_blocks_and_survives() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(401, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[serra_zealot()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, serra_zealot());
    pass_until(&mut engine, stack_is_empty);

    let zealot = on_battlefield(&engine, p0, serra_zealot()).expect("the Zealot resolved");
    let elves = on_battlefield(&engine, p1, quiet_creature()).expect("the Elf is across the table");
    assert_eq!(pt(&engine, zealot), (1, 1), "a printed 1/1");
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and so is the Elf it will block"
    );
    assert!(
        keywords(&engine, zealot).contains(KeywordSet::FIRST_STRIKE),
        "the printed keyword reaches the permanent"
    );

    // The one mana was spent casting it and nothing tapped it, so the Zealot
    // is an untapped creature and may block the Elf on the very next turn
    // (CR 509.1a) — summoning sickness restricts attacking and {T}, not this.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, Defender::Player(p0))],
            },
        )
        .expect("a 1/1 that has been in play since the game began may attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the declaration")
    };
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == zealot && option.attackers.contains(&elves)),
        "the Zealot can block the Elf: {blockers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(zealot, elves)],
            },
        )
        .expect("the pairing the offer named");

    // Past both damage steps (CR 510.2 and 510.4) and into the end step.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "the Elf died in the first strike damage step: the Zealot's one \
         damage was dealt before the Elf's turn to deal any"
    );
    assert!(
        on_battlefield(&engine, p0, serra_zealot()).is_some(),
        "and the Zealot is still on the battlefield — without first strike a \
         1/1 blocking a 1/1 trades, so a survivor here is the keyword acting \
         and not merely printed"
    );
}
