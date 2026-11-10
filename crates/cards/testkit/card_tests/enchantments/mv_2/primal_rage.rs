//! `cards/enchantments/mv_2/primal_rage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Primal Rage — {1}{G} enchantment: "Creatures you control have trample."
///
/// The static is `Filter::YOUR_CREATURE`, so the reading worth playing is the
/// one that tells this seat's creatures from the opponent's: an Elf sits under
/// each seat and only the one this side controls may carry the keyword once the
/// enchantment has resolved. The Elf cast *afterwards* is the other half of the
/// same claim — a grant that reached only what stood on the battlefield at
/// resolution would read the printed card rather than the board the static
/// lives on — and the enchantment itself is no creature, so it must not carry
/// the keyword it hands out.
#[test]
fn primal_rage_grants_trample_to_the_creatures_of_its_controller_including_a_latecomer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(131, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[primal_rage(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "nothing has granted a keyword yet"
    );

    // The Elf is named as the printing kept back: it prints its own
    // `{T}: Add {G}`, so "three Forests" is a claim about the Forests and not
    // about a board where a creature quietly paid in (rule 11).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Elf kept untapped"
    );
    cast_with_floating(&mut engine, p0, primal_rage());
    pass_until(&mut engine, stack_is_empty);

    let rage = on_battlefield(&engine, p0, primal_rage()).expect("Primal Rage resolved");
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "\"creatures you control have trample\" — the Elf under this seat carries it"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the Elf across the table does not: \"you control\" is read and not skipped"
    );
    assert!(
        !keywords(&engine, rage).contains(KeywordSet::TRAMPLE),
        "the enchantment grants the keyword to creatures; it is not one itself"
    );

    // The green the cast left behind, spent on a creature that arrives after
    // the static is already on the battlefield.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{G}} out of three green leaves exactly one for the Elf"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "the second Elf resolved beside the first");
    assert!(
        elves
            .iter()
            .all(|id| keywords(&engine, *id).contains(KeywordSet::TRAMPLE)),
        "a creature that enters under the static is projected against it too"
    );
    assert!(
        !keywords(
            &engine,
            on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is still out")
        )
        .contains(KeywordSet::TRAMPLE),
        "and the arrival changes nothing across the table"
    );
}
