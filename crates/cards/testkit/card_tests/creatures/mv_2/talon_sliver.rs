//! `cards/creatures/mv_2/talon_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Talon Sliver — {1}{W} 1/1 Sliver: "All Sliver creatures have first strike."
///
/// The grant is a continuous effect over a subtype filter, so the scenario reads
/// it where only the layers can be seen: nothing on this board has first strike
/// before the cast, and the Sliver that then arrives has it — from its own printed
/// static, which reaches its own source the way the card says it does. The
/// Llanowar Elves standing beside it are the control, a creature under the same
/// controller that the subtype filter has to decline, and the last claim pins the
/// static as a static: it is projected, never offered as something to press.
#[test]
fn talon_sliver_arrives_with_first_strike_and_grants_it_to_no_creature_without_its_subtype() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[talon_sliver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "no Sliver is on the battlefield yet, so nothing grants anything"
    );

    // {1}{W} off the two Plains, with the Elves named as the printing kept back:
    // they are the creature this test reads afterwards, and a mana creature is a
    // mana route too (#159).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains tapped and no creature of mine tapped for mana"
    );
    cast_with_floating(&mut engine, p0, talon_sliver());
    pass_until(&mut engine, stack_is_empty);

    let sliver = on_battlefield(&engine, p0, talon_sliver()).expect("the Sliver resolved");
    assert!(
        types(&engine, sliver).contains(TypeSet::CREATURE),
        "it arrives as the creature it prints"
    );
    assert_eq!(pt(&engine, sliver), (1, 1), "and with the printed 1/1 body");
    // Deliberately no "not yet": the Sliver **is** a Sliver creature, so its
    // own static reaches it the moment it is on the battlefield, and the two
    // assertions at the end of this test are where that is read. A grant is
    // separated from a printed keyword by the Elf beside it instead.
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "and the creature without the subtype still has nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the spell resolved and the seat holds priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == sliver),
        "a static is never offered as an activation: {:?}",
        legal.abilities
    );

    let armed = keywords(&engine, sliver);
    assert!(
        armed.contains(KeywordSet::FIRST_STRIKE),
        "\"All Sliver creatures have first strike\" — and this is one, so the \
         grant reaches its own source: {armed:?}"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "a creature without the subtype gets nothing, on the very same side of \
         the table"
    );
}
