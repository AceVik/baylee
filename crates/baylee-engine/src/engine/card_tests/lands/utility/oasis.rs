//! `cards/lands/utility/oasis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Oasis — "{T}: Prevent the next 1 damage that would be dealt to target
/// creature this turn."
///
/// The shield is worth exactly one point (CR 615.1): a Hill Giant (3/3)
/// eats a Lightning Bolt for 2 and lives, and the journal's `DamageDealt`
/// carries the after-prevention amount. The unshielded Giant one table over
/// takes all 3 and dies, so the survival above cannot pass on a Bolt that
/// never dealt anything.
#[test]
fn oasis_prevents_the_next_point_of_damage_to_a_creature_and_no_more() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[oasis(), hill_giant(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("the Giant is seated");

    activate(&mut engine, p0, oasis(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the shield's target, got {:?}", engine.pending())
    };
    assert!(options.contains(&giant), "the seated Giant may be shielded");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let before = engine.journal().entries().len();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(
                e.event,
                crate::event::GameEvent::DamageDealt {
                    target: crate::event::DamageTarget::Object(hit),
                    amount: 2,
                    ..
                } if hit == giant
            )),
        "the Bolt's 3 damage, 1 of it prevented, is journalled as 2"
    );
    assert!(
        on_battlefield(&engine, p0, hill_giant()).is_some(),
        "2 marked on a 3/3 is not lethal"
    );

    // Unshielded: all 3 are marked and the Giant dies.
    let mut bare = Duel::new(SEED, mountain())
        .battlefield(0, &[hill_giant(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut bare);
    reach_main_phase(&mut bare, p0);
    let giant = on_battlefield(&bare, p0, hill_giant()).expect("the Giant is seated");
    let before = bare.journal().entries().len();
    cast_from_hand(&mut bare, p0, lightning_bolt());
    bare.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![giant],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut bare, stack_is_empty);
    assert!(
        bare.journal().entries()[before..].iter().any(|e| matches!(
            e.event,
            crate::event::GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Object(hit),
                amount: 3,
                ..
            } if hit == giant
        )),
        "unshielded, the journalled amount is the Bolt's own 3"
    );
    assert!(
        in_graveyard(&bare, p0, hill_giant()).is_some(),
        "3 is lethal on a 3/3"
    );
}
