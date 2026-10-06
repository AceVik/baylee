//! `cards/creatures/mv_4/aven_cloudchaser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aven Cloudchaser is a `{3}{W}` 2/2 Bird Soldier with flying whose one
/// printed trigger is "When this creature enters, destroy target
/// enchantment". The board is built so the offer cannot be read off the card
/// file: while the question stands the battlefield holds four Plains, the
/// Cloudchaser itself and a creature across the table, so `options` naming
/// exactly the one enchantment is what reads `Filter::ENCHANTMENT` rather
/// than `Filter::Any` — and the card landing in its owner's graveyard is what
/// says the destroy actually resolved instead of merely being aimed. Paying
/// `{3}{W}` out of four tapped Plains is the other half: the 2/2 with flying
/// that follows is the body the card prints, and no harness seeded it.
#[test]
fn aven_cloudchaser_destroys_the_only_enchantment_on_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[aven_cloudchaser()])
        // One enchantment and one creature across the table, so the filter
        // has something to reach and something to decline.
        .battlefield(1, &[their_enchantment(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let theirs = on_battlefield(&engine, p1, their_enchantment()).expect("the enchantment is out");
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("and a creature beside it");

    // `{3}{W}` out of the four Plains, which are spent before anything asks.
    cast_from_hand(&mut engine, p0, aven_cloudchaser());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the controller of the trigger answers it");
    assert_eq!(
        options,
        vec![theirs],
        "the one enchantment on a board of Plains, Elves and the Cloudchaser"
    );
    assert!(
        !options.contains(&elf),
        "a creature is no enchantment, not even the one standing next to it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the enchantment was the option the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "\"destroy target enchantment\": it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and destroying a permanent puts it in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "the permanent the destroy never named never moved"
    );

    let chaser = on_battlefield(&engine, p0, aven_cloudchaser()).expect("the Cloudchaser landed");
    assert_eq!(pt(&engine, chaser), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, chaser).contains(KeywordSet::FLYING),
        "and the flying line it prints"
    );
}
