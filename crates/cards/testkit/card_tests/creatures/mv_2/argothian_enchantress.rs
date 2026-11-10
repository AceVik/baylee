//! `cards/creatures/mv_2/argothian_enchantress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Argothian Enchantress — {1}{G} 0/1 Human Druid — prints two lines: shroud,
/// and "Whenever you cast an enchantment spell, draw a card."
///
/// One Rancor plays both. It is an enchantment spell whose "enchant creature"
/// is a target choice, so that menu is where shroud is read — and shroud is
/// absolute (CR 702.18b), which is why the Enchantress has to be missing from
/// its *own* controller's Aura while the Elf beside it is offered. The cast is
/// then what the trigger counts, and the Lightning Greaves are the control on
/// the other side of that filter: an artifact spell cast while the Enchantress
/// is on the battlefield, drawing nothing.
#[test]
fn argothian_enchantress_shrouds_itself_and_draws_only_for_an_enchantment() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        // Five Forests, because three spells are cast off one pool: {1}{G} for
        // the Enchantress, {2} for the Greaves and {G} for the Rancor.
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[argothian_enchantress(), rancor(), lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    // The five Forests, with the Elf kept back so the pool holds exactly what
    // the lands made and the Elf's own green cannot cover a miscount.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, argothian_enchantress());
    pass_until(&mut engine, stack_is_empty);
    let enchantress = on_battlefield(&engine, p0, argothian_enchantress())
        .expect("the Enchantress resolved from hand");
    assert_eq!(pt(&engine, enchantress), (0, 1), "the body the card prints");
    assert!(
        keywords(&engine, enchantress).contains(KeywordSet::SHROUD),
        "shroud is on the permanent, and the only reading worth playing is \
         the one a target menu makes"
    );

    // The control: an artifact spell, cast while the Enchantress is watching.
    let library_before = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "\"whenever you cast an *enchantment* spell\" — an artifact cast is \
         not one, so nothing was drawn"
    );

    // The subject: an enchantment spell that also has to choose a target.
    cast_with_floating(&mut engine, p0, rancor());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster picks the creature the Aura holds");
    assert!(
        options.contains(&elf),
        "the Elf is a legal target for the Aura: {options:?}"
    );
    assert!(
        !options.contains(&enchantress),
        "shroud: the Enchantress may not be the target of a spell, and that \
         includes its own controller's Aura (CR 702.18b): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"whenever you cast an enchantment spell, draw a card\" — one card \
         for the Aura, where the Greaves moved none"
    );
    assert!(
        on_battlefield(&engine, p0, rancor()).is_some(),
        "the Aura resolved onto the battlefield"
    );
    assert_eq!(
        pt(&engine, elf),
        (3, 1),
        "+2/+0 on the creature it reached, so the draw above is the cast of a \
         spell that actually landed"
    );
}
