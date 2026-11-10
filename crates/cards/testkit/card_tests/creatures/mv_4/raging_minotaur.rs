//! `cards/creatures/mv_4/raging_minotaur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raging Minotaur is a `{2}{R}{R}` 3/3 Minotaur Berserker whose whole printed
/// text is haste, and haste is invisible in a board state: the keyword means
/// nothing until the declare-attackers question, where it is the difference
/// between a creature that arrived this turn and one that did not (CR 302.6).
/// So two creatures are cast in the *same* main phase — the Minotaur and a
/// Llanowar Elves, which prints no haste — and the offer has to name one and
/// decline the other. Reading the card file instead would prove nothing, since
/// the Elf satisfies every characteristic written there.
#[test]
fn raging_minotaur_attacks_the_turn_it_arrives_where_a_fresh_elf_cannot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[raging_minotaur(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five lands, five mana, all inside this one main phase: the Forest's {G}
    // pays the Elf and the four Mountains pay {2}{R}{R} for the Minotaur
    // (CR 500.5). Neither creature is on the board while the tapping happens,
    // so nothing on it has a mana ability of its own to count (rule 11).
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        (
            pool.available(ManaColor::Red),
            pool.available(ManaColor::Green)
        ),
        (4, 1),
        "four Mountains and one Forest, and nothing else on the board"
    );

    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, raging_minotaur());
    pass_until(&mut engine, stack_is_empty);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf resolved");
    let minotaur = on_battlefield(&engine, p0, raging_minotaur()).expect("the Minotaur resolved");
    assert_eq!(pt(&engine, minotaur), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, minotaur).contains(KeywordSet::HASTE),
        "the printed haste reaches the permanent"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::HASTE),
        "and the Elf beside it is the control: no haste is printed on it"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&minotaur),
        "it entered this turn and may attack anyway, which is what haste is \
         for (CR 302.6): {attackers:?}"
    );
    assert!(
        !attackers.contains(&elf),
        "the Elf entered in the same main phase, is untapped, and has no \
         haste — so summoning sickness is what the offer is reading and the \
         line above is not an offer that would name anything: {attackers:?}"
    );
}
