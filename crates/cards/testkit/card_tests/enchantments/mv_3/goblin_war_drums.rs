//! `cards/enchantments/mv_3/goblin_war_drums.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin War Drums is one printed sentence — "Creatures you control have
/// menace" — and the whole card lives in the two words that narrow it. So
/// the board carries both halves of each: a Sol Ring under the same seat
/// (which a `Filter::Any` would have granted menace to, as an artifact and
/// no creature) and an Elf across the table (which a static that read "the
/// table" instead of "you" would have granted it to). The grant is read off
/// the projected characteristics *after* the enchantment has been cast and
/// resolved, because a static that never registered grants nothing to
/// anybody.
#[test]
fn goblin_war_drums_gives_menace_to_the_creatures_its_controller_has_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4211, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[goblin_war_drums()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::MENACE),
        "nothing is enchanting the board yet"
    );

    // {2}{R} off the three Mountains, with the Elf and the Sol Ring named as
    // the two things kept back: the pool the enchantment is paid from is then
    // exactly the lands this test goes on to read, and neither of the
    // permanents about to be asked about has moved for a reason of its own.
    tap_mana_where(&mut engine, p0, |id| id != elves && id != rock);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains and nothing else: the Elf and the Sol Ring made no mana"
    );
    cast_with_floating(&mut engine, p0, goblin_war_drums());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, goblin_war_drums()).is_some()
    });
    let drums = on_battlefield(&engine, p0, goblin_war_drums()).expect("the enchantment resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{R}} came out of the pool"
    );

    assert!(
        keywords(&engine, elves).contains(KeywordSet::MENACE),
        "\"creatures you control have menace\""
    );
    assert!(
        !keywords(&engine, rock).contains(KeywordSet::MENACE),
        "\"creatures you control\" is not \"permanents you control\": the Sol \
         Ring is an artifact and no creature"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::MENACE),
        "\"you control\" is not the table: the Elf across it is a creature \
         too, and the static never reaches it"
    );
    assert!(
        !keywords(&engine, drums).contains(KeywordSet::MENACE),
        "the enchantment grants the keyword, it does not keep it"
    );
}
