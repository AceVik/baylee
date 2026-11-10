//! `cards/creatures/mv_3/raging_cougar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raging Cougar is `{2}{R}` for a 2/2 Cat whose whole printed text is haste,
/// so the only play that proves anything is the one summoning sickness would
/// otherwise forbid: cast it in a main phase and offer it as an attacker in
/// the same turn. The Llanowar Elves cast beside it is what makes the offer
/// mean something — it is an untapped 1/1 that entered on the same turn, and
/// it is missing from `ChooseAttackers` for exactly the reason the Cougar is
/// present, so the list is not simply everything on the board.
#[test]
fn raging_cougar_attacks_the_turn_it_arrives_while_a_sick_elf_cannot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), mountain(), mountain(), mountain()])
        .hand(0, &[raging_cougar(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana before the cast: `can_afford` reads the pool and not the untapped
    // lands. The Forest is named as the printing kept back because it is what
    // pays for the Elf afterwards.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains in the pool and the Forest still standing"
    );
    cast_with_floating(&mut engine, p0, raging_cougar());
    pass_until(&mut engine, stack_is_empty);

    let cougar = on_battlefield(&engine, p0, raging_cougar()).expect("the Cougar resolved");
    assert_eq!(pt(&engine, cougar), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, cougar).contains(KeywordSet::HASTE),
        "the one line the card prints reaches the permanent"
    );

    // The control, cast on the very same turn: an untapped creature with no
    // summoning-sickness exemption of its own.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf resolved");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "the control prints no haste of its own"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&cougar),
        "a 2/2 cast this turn is offered as an attacker, which is haste (CR 702.10b): {attackers:?}"
    );
    assert!(
        !attackers.contains(&elves),
        "and the untapped Elf cast the same turn is not — summoning sickness \
         (CR 302.6) is the only thing keeping it out: {attackers:?}"
    );
}
