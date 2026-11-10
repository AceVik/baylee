//! `cards/creatures/mv_6/kavu_mauler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "6ffbcaba-5437-4fb6-a2d6-e94b1e6dc1d2"

/// Kavu Mauler — {4}{G}{G}, a 4/4 Kavu with trample: "Whenever this creature
/// attacks, it gets +1/+1 until end of turn for each other attacking Kavu."
///
/// One attack declaration reads all three words of that sentence at once. Two
/// Maulers and a non-Kavu Elf attack together, and both Maulers read 5/5: a
/// filter missing `Another` would count the Mauler itself and land on 6/6, one
/// missing the Kavu subtype would count the Elf beside it and land on 6/6 too,
/// and an effect that pumped the whole attacking team would move the Elf off
/// its printed 1/1. The combat damage shows the pump was a real body (five and
/// five, not four and four) and the next turn shows it was a duration.
#[test]
fn kavu_mauler_counts_other_attacking_kavu_and_not_itself_the_elf_or_any_turn_after() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board: Vec<CardIndex> = vec![forest(); 12];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[kavu_mauler(), kavu_mauler()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf is the one mana source kept back, because it is one of the three
    // attackers this test reads afterwards: a creature tapped for mana may not
    // attack. Twelve Forests are exactly {4}{G}{G} twice over.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve tapped Forests and an untapped Elf, which is every source on \
         the board"
    );
    cast_with_floating(&mut engine, p0, kavu_mauler());
    pass_until(&mut engine, |e| at_rest(e, p0));
    cast_with_floating(&mut engine, p0, kavu_mauler());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let maulers = all_on_battlefield(&engine, p0, kavu_mauler());
    assert_eq!(maulers.len(), 2, "both Maulers resolved onto the table");
    let (first, second) = (maulers[0], maulers[1]);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(
        pt(&engine, first),
        (4, 4),
        "a printed 4/4 before it attacks"
    );
    assert!(
        keywords(&engine, first).contains(KeywordSet::TRAMPLE),
        "and the printed trample reaches the permanent"
    );

    // Summoning sickness (CR 302.6): both were cast this turn, so the attack
    // declaration has to wait for a turn that began with them on the table.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&first) && attackers.contains(&second) && attackers.contains(&elf),
        "all three are untapped and past summoning sickness: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (first, Defender::Player(p1)),
                    (second, Defender::Player(p1)),
                    (elf, Defender::Player(p1)),
                ],
            },
        )
        .expect("the three attackers came out of the list that offered them");

    // Walk the combat out to the end step, where the printed "until end of
    // turn" is still in force (CR 514.2 ends it in the cleanup step) and the
    // damage has already been dealt: neither assertion below can be read off a
    // board that stopped short of the damage step.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending) && e.state().turn.active == p0
    });

    assert_eq!(
        pt(&engine, first),
        (5, 5),
        "+1/+1 for the other attacking Kavu, and for nothing else: 6/6 would \
         mean it counted itself, or the Elf beside it"
    );
    assert_eq!(pt(&engine, second), (5, 5), "and the other way round");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the pump lands on the Mauler that triggered, not on the attacking team"
    );
    assert_eq!(
        engine.state().players[1].life,
        9,
        "five and five through unblocked plus the Elf's one: a board where the \
         trigger never resolved would have taken nine, not eleven"
    );

    // "until end of turn": a whole turn later both are the 4/4 they were
    // printed as, so the +1/+1 was a duration and not a counter or a static.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, first),
        (4, 4),
        "the grant lasted the turn it was made in and no longer"
    );
    assert_eq!(pt(&engine, second), (4, 4), "on both of them");
    assert!(
        on_battlefield(&engine, p0, kavu_mauler()).is_some(),
        "and the Mauler is still standing, so the pump left rather than the \
         creature"
    );
}
