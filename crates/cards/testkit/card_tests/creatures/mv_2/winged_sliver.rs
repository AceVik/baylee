//! `cards/creatures/mv_2/winged_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Winged Sliver prints a single static line: "{1}{U} — 1/1 — All
/// Sliver creatures have flying." Two things are testable about this and each
/// needs its own witness: that the ability reaches the Sliver *itself*,
/// even though it comes from its own text, and that it only reaches
/// Slivers. The Elf under the same control and the Elf across the
/// table are therefore both there — a filter that had lost
/// `HasSubtype(SLIVER)` and would let every creature fly would be
/// immediately visible on both, and a filter that silently reads only
/// its own side, on the second. Payment is made with two Islands,
/// so that the Elf on which it is read does not have to be tapped
/// for its own mana.
#[test]
fn winged_sliver_grants_flying_to_slivers_and_to_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[winged_sliver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "no Sliver is on the table, so there is nothing to grant"
    );

    // The two Islands pay {1}{U}; the Elf is left named,
    // because it is the witness on which the subtype filter is read.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands in the mana pool and the Elf explicitly not among them"
    );
    cast_with_floating(&mut engine, p0, winged_sliver());
    pass_until(&mut engine, stack_is_empty);

    let sliver = on_battlefield(&engine, p0, winged_sliver()).expect("der Sliver ist angekommen");
    assert_eq!(pt(&engine, sliver), (1, 1), "der gedruckte 1/1-Körper");
    assert!(
        keywords(&engine, sliver).contains(KeywordSet::FLYING),
        "\"All Sliver creatures have flying\" — der Sliver ist einer davon, \
         also erreicht seine eigene Statik ihn selbst"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "a creature without the subtype gets nothing, no matter which \
         side it is on"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and the static reaches no further across the table: the filter is the \
         subtype and not the battlefield"
    );
}
