//! `cards/creatures/mv_2/dwalin_weaponmaster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dwalin, Weaponmaster — {1}{R/W} — 2/1 legendary Dwarf Warrior with
/// "First strike". He is cast rather than seeded onto a board, and the
/// keyword is proven where it is the only thing that can be seen: a 1/1
/// Llanowar Elves blocking him. Dwalin's two damage is dealt in the
/// first-strike damage step, so the Elf dies before it deals the single
/// point that would also be lethal to a 2/1 — without the keyword the two
/// trade, which is exactly the reading asserted against. The hone-counter
/// sentence is the `Coverage::Partial` half and stays off the card, so
/// nothing else is claimed here.
#[test]
fn dwalin_kills_what_blocks_him_before_it_can_kill_him_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), plains()])
        .hand(0, &[dwalin_weaponmaster()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, dwalin_weaponmaster());
    pass_until(&mut engine, stack_is_empty);
    let dwalin = on_battlefield(&engine, p0, dwalin_weaponmaster()).expect("Dwalin resolved");
    assert!(
        keywords(&engine, dwalin).contains(KeywordSet::FIRST_STRIKE),
        "the printed keyword, as the layer system projects it"
    );

    // He was cast this turn, so the attack has to wait for the next one.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the attacker declaration")
    };
    let defender = defenders
        .into_iter()
        .next()
        .expect("the other seat is the one thing to attack");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dwalin, defender)],
            },
        )
        .expect("an untapped, unsick Dwalin may attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is still standing");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, dwalin)],
            },
        )
        .expect("the Elf may block him");

    // The Elf's death is the end of the first-strike damage step, which is
    // also where the loop stops caring about anything behind it.
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "his two damage is more than a 1/1 has"
    );
    assert!(
        on_battlefield(&engine, p0, dwalin_weaponmaster()).is_some(),
        "and the Elf's damage is dealt in no step at all: one point of it \
         would have been lethal to a 2/1, so a Dwalin that had merely traded \
         would be lying in the graveyard beside it"
    );
}
