//! `cards/enchantments/mv_3/temur_ascendancy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Temur Ascendancy: the haste it grants, on a creature that has just
/// arrived.
///
/// The "power 4 or greater" draw trigger is refused by name, so the static is
/// the card here — and it is a second printing of the same sentence Maelstrom
/// Wanderer carries, which is why this one is asserted on the keyword and on
/// the attacker list rather than on both again.
#[test]
fn temur_ascendancy_gives_a_freshly_cast_creature_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(410, forest())
        .battlefield(0, &[temur_ascendancy(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf arrived this turn");
    assert!(
        keywords(&engine, elf).contains(KeywordSet::HASTE),
        "\"Creatures you control have haste\""
    );
}

/// Temur Ascendancy's second line is the one Garruk's Uprising prints as a
/// gift rather than a choice: "Whenever a creature you control with power 4
/// or greater enters, **you may** draw a card." The "may" is answered here
/// rather than assumed — a `MayDo` nobody is asked resolves into nothing —
/// and a 1/1 entering asks no question at all, which is the half that says
/// the power predicate is on the trigger and not on the draw.
#[test]
fn temur_ascendancy_offers_its_draw_only_for_a_four_power_creature() {
    let p0 = PlayerId::new(0);

    let mut small = Duel::new(9303, forest())
        .battlefield(0, &[temur_ascendancy(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut small);
    reach_main_phase(&mut small, p0);
    let before = library_size(&small, p0);
    cast_from_hand(&mut small, p0, llanowar_elves());
    pass_until(&mut small, stack_is_empty);
    assert!(
        on_battlefield(&small, p0, llanowar_elves()).is_some(),
        "the Elf arrived"
    );
    assert_eq!(
        library_size(&small, p0),
        before,
        "a 1/1 entering triggers nothing, so nothing was asked and nothing drawn"
    );

    let mut big = Duel::new(9304, forest())
        .battlefield(
            0,
            &[
                temur_ascendancy(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut big);
    reach_main_phase(&mut big, p0);
    let before = library_size(&big, p0);
    cast_from_hand(&mut big, p0, rootbreaker_wurm());
    pass_until(&mut big, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    big.apply(p0, PlayerAction::YesNo(true))
        .expect("the \"you may\" is answered");
    pass_until(&mut big, stack_is_empty);
    assert_eq!(
        library_size(&big, p0),
        before - 1,
        "the 6/6 entering asked, and the answer was yes"
    );
}
