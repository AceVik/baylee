//! `cards/enchantments/mv_3/glorious_anthem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glorious Anthem — {1}{W}{W}: "Creatures you control get +1/+1."
///
/// The card is a static over a *side* of the table, so the scenario needs a
/// creature on each: two Llanowar Elves under seat 0 that must both read
/// 2/2, and one across it that must stay the printed 1/1 — an anthem that
/// had lost `Filter::YOUR_CREATURE` would pump the whole table and still
/// satisfy the first half. The Elves are the read-out and are named as the
/// printing kept back, so the three Plains are the only thing that pays and
/// neither `(2, 2)` can be a tapped creature's own doing.
#[test]
fn glorious_anthem_pumps_every_creature_its_controller_has_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[glorious_anthem()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves on this side of the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    for elf in &mine {
        assert_eq!(pt(&engine, *elf), (1, 1), "printed 1/1s before the Anthem");
    }
    assert_eq!(pt(&engine, theirs), (1, 1), "and so is the one across it");

    // Three Plains pay {1}{W}{W}; both Elves are kept off the mana so the
    // creatures the pump is read back on are not sources that tapped.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains in the pool and no Elf's {{G}} among them"
    );
    cast_with_floating(&mut engine, p0, glorious_anthem());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, glorious_anthem()).is_some(),
        "the Anthem resolved onto the battlefield"
    );
    for elf in &mine {
        assert_eq!(
            pt(&engine, *elf),
            (2, 2),
            "every creature this seat controls is +1/+1"
        );
    }
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"you control\" is not the whole table"
    );
}
