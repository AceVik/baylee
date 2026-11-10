//! `cards/creatures/mv_2/monk_realist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Monk Realist is `{1}{W}` for a 1/1 whose whole text is "When this creature
/// enters, destroy target enchantment." The scenario puts the enchantment
/// across the table and an Elf and a Forest beside it, so a
/// `Filter::ENCHANTMENT` that had widened to "any permanent" would show up in
/// the offer — and the target question is read *after* the body has landed,
/// which is what makes this an enters-trigger rather than the cast itself.
#[test]
fn monk_realist_enters_and_destroys_an_enchantment_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[monk_realist()])
        .battlefield(1, &[luminarch_ascension(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ascension = on_battlefield(&engine, p1, luminarch_ascension())
        .expect("the enchantment is across the table");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf is across the table");
    let land = on_battlefield(&engine, p1, forest()).expect("a Forest is across the table");

    // {1}{W} off the two Plains, mana into the pool first: the offer reads the
    // pool and not the untapped lands.
    cast_from_hand(&mut engine, p0, monk_realist());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    assert!(
        on_battlefield(&engine, p0, monk_realist()).is_some(),
        "the body landed before its enters-trigger asked anything"
    );

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the target choice")
    };
    assert_eq!(player, p0, "the creature's controller aims its own trigger");
    assert!(
        options.contains(&ascension),
        "\"target enchantment\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&elf) && !options.contains(&land),
        "an Elf and a Forest are permanents and no enchantments: {options:?}"
    );
    assert_eq!(options.len(), 1, "one enchantment, one option: {options:?}");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ascension],
            },
        )
        .expect("the enchantment the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, luminarch_ascension()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, luminarch_ascension()).is_some(),
        "and it is in its owner's graveyard, which is where a destroyed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some()
            && on_battlefield(&engine, p1, forest()).is_some(),
        "the permanents the ability did not name never moved"
    );
    let realist = on_battlefield(&engine, p0, monk_realist()).expect("still on the battlefield");
    assert_eq!(
        pt(&engine, realist),
        (1, 1),
        "a printed 1/1 that traded nothing of itself for the enchantment"
    );
}
