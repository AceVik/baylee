//! `cards/enchantments/mv_2/circle_of_protection_red.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Circle of Protection: Red against a red pinger killed in response: its
/// ability still deals the damage, from the source it was, and that source
/// may be chosen and is prevented from (CR 609.7a — "any object referred to
/// by an object on the stack … even if that object is no longer in the zone
/// it used to be in"). The 3 the Artillery deals its own controller is the
/// control: the source dealt damage, and only the shielded player was
/// spared it.
#[test]
fn circle_of_protection_prevents_a_pinger_killed_in_response() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(47, forest())
        .battlefield(0, &[circle_of_protection_red(), plains()])
        .battlefield(1, &[orcish_artillery()])
        .start();
    keep_mulligans(&mut engine);
    let artillery = on_battlefield(&engine, p1, orcish_artillery()).expect("deployed");
    reach_their_main_phase(&mut engine, p1);

    activate(&mut engine, p1, orcish_artillery(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("any target, and a player is one");
    let selected = baylee_core::ids::DamageSourceRef {
        object: artillery,
        version: engine
            .state()
            .object(artillery)
            .expect("source exists")
            .version,
    };
    bury(&mut engine, &[artillery]);
    assert_eq!(
        in_graveyard(&engine, p1, orcish_artillery()),
        Some(artillery)
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, circle_of_protection_red(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0);
    assert_eq!(
        options,
        vec![selected],
        "the only red source: the pinger in the graveyard its ability names"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .expect("off the list");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20, "the 2 was prevented");
    assert_eq!(
        engine.state().players[1].life,
        17,
        "and the 3 to its controller was dealt"
    );
    assert!(engine.state().shields.is_empty(), "the shield was used");
}
