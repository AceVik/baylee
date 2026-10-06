//! `cards/creatures/mv_4/anaba_spirit_crafter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Anaba Spirit Crafter is a {2}{R}{R} 1/3 Minotaur Shaman whose whole text is
/// "Minotaur creatures get +1/+0" — a static with neither "you control" nor
/// "another" in it, so it changes the Crafter's own body as it lands and moves
/// no creature that is not a Minotaur.
///
/// The two bystanders are read *before* the cast and compared after it: the
/// claim is that the static moved a Minotaur and left everything else exactly
/// where it found it, so an Elf Druid and a Human Soldier on the same
/// battlefield are as tall afterwards as they were before, while the arriving
/// 1/3 Minotaur reads (2, 3).
#[test]
fn anaba_spirit_crafter_pumps_minotaurs_and_leaves_the_other_creatures_where_they_were() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
                drannith_magistrate(),
            ],
        )
        .hand(0, &[anaba_spirit_crafter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let soldier =
        on_battlefield(&engine, p0, drannith_magistrate()).expect("the Magistrate is out");
    let elf_before = pt(&engine, elf);
    let soldier_before = pt(&engine, soldier);
    assert_eq!(
        elf_before,
        (1, 1),
        "a printed 1/1 before the Crafter arrives"
    );
    assert!(
        soldier_before.0 > 0,
        "the comparison below is only worth making against a creature with a \
         power to take away: {soldier_before:?}"
    );

    cast_from_hand(&mut engine, p0, anaba_spirit_crafter());
    pass_until(&mut engine, stack_is_empty);

    let crafter =
        on_battlefield(&engine, p0, anaba_spirit_crafter()).expect("the Crafter resolved");
    assert_eq!(
        pt(&engine, crafter),
        (2, 3),
        "the printed 1/3 Minotaur plus its own +1/+0 — \"Minotaur creatures\" is \
         not \"other Minotaurs\" and not \"Minotaurs you control\""
    );
    assert_eq!(
        pt(&engine, elf),
        elf_before,
        "an Elf Druid is no Minotaur, so the static left it exactly as it found it"
    );
    assert_eq!(
        pt(&engine, soldier),
        soldier_before,
        "and a Human Soldier is no Minotaur either: reading the subtype is the \
         whole of what the ability does"
    );
}
