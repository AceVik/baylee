//! `cards/enchantments/mv_2/steely_resolve.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Steely Resolve: the type is chosen as it enters, and only that type is
/// untargetable.
///
/// `EnterModifier::ChooseSubtype` plus a filter that reads the choice is the
/// whole card, and the pair is only proved by a board with a creature of the
/// chosen type and one of another: the Elf gains shroud and the Beast beside
/// it does not.
#[test]
fn steely_resolve_shrouds_only_the_type_it_was_given() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(401, forest())
        .battlefield(
            0,
            &[forest(), forest(), llanowar_elves(), rootbreaker_wurm()],
        )
        .hand(0, &[steely_resolve()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is seated");
    cast_from_hand(&mut engine, p0, steely_resolve());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::ELF),
        )
        .expect("Elf is a creature type");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::SHROUD),
        "the Elf is of the chosen type"
    );
    assert!(
        !keywords(&engine, wurm).contains(KeywordSet::SHROUD),
        "and the Wurm is not, so the choice is read rather than ignored"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&wurm) && !options.contains(&elf),
        "shroud is what the offer reads, not a keyword nobody asks about: \
         {options:?}"
    );
}
