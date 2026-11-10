//! `cards/instants/mv_6/pull_under.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pull Under is a {5}{B} instant under `Coverage::Implemented` that gives target creature -5/-5 until end of turn.
/// When cast, creatures on the battlefield are presented as legal targets while non-creatures are excluded.
/// Targeting a 6/6 creature reduces its projected power and toughness down to 1/1 without destroying it.
/// Bystanders not selected by the spell remain unaffected.
#[test]
fn pull_under_gives_target_creature_minus_five_minus_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[pull_under()])
        .battlefield(1, &[rootbreaker_wurm(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, pull_under()).expect("Pull Under is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{5}}{{B}}"
    );

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("their Wurm is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(pt(&engine, wurm), (6, 6), "Wurm starts as 6/6");
    assert_eq!(pt(&engine, elf), (1, 1), "Elf starts as 1/1");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Swamps produce six mana"
    );
    cast_with_floating(&mut engine, p0, pull_under());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "caster chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&wurm) && options.contains(&elf),
        "both creatures are valid targets: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("targeting Wurm is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (1, 1),
        "Wurm received -5/-5, becoming a 1/1"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "Elf was not targeted and remains a 1/1"
    );
    assert!(
        in_graveyard(&engine, p0, pull_under()).is_some(),
        "Pull Under went to graveyard upon resolution"
    );
}
