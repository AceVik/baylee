//! `cards/artifacts/mv_4/blood_clock.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn blood_clock() -> CardIndex {
    card_index("ef987ddb-25f6-4895-8f83-045254745ea4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Blood Clock (ef987ddb-25f6-4895-8f83-045254745ea4) — {4} Artifact:
/// "At the beginning of each player's upkeep, that player returns a permanent
///  they control to its owner's hand unless they pay 2 life."
///
/// On p0's upkeep, p0 pays 2 life (20 -> 18) and keeps all permanents; on p1's
/// upkeep, p1 declines to pay life (stays 20) and returns a permanent they
/// control to hand.
#[test]
fn blood_clock_each_upkeep_pays_two_life_or_bounces_own_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blood_clock(), grizzly_bears()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);

    let clock = on_battlefield(&engine, p0, blood_clock()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let their_forest = on_battlefield(&engine, p1, forest()).unwrap();

    // 1. p0's upkeep: Blood Clock triggers; p0 pays 2 life -> keeps both permanents.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().turn.active, p0);
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    assert_eq!(engine.pending().asked(), Some(p0));

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 18, "p0 paid 2 life");
    assert_eq!(
        engine.state().object(clock).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Battlefield
    );

    // 2. p1's upkeep: Blood Clock triggers; p1 declines -> must bounce one of their permanents.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    assert_eq!(engine.pending().asked(), Some(p1));

    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards for bounce, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&their_elf) && options.contains(&their_forest),
        "p1's permanents are offered: {options:?}"
    );
    assert!(
        !options.contains(&clock) && !options.contains(&bears),
        "p0's permanents are excluded: {options:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, p1), 20, "p1 did not pay life");
    assert_eq!(
        engine.state().object(their_elf).unwrap().zone,
        Zone::Hand,
        "chosen permanent returned to owner's hand"
    );
    assert_eq!(
        engine.state().object(their_forest).unwrap().zone,
        Zone::Battlefield
    );
}
