//! `cards/artifacts/mv_1/triangle_of_war.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn triangle_of_war() -> CardIndex {
    card_index("6d4daf9c-e2f4-4d0e-b2e4-ded04d644b11")
}

/// `Triangle of War` ({1}, Artifact, `Coverage::Implemented`) sacrifices itself
/// for `{2}` so target creature you control fights target creature an opponent
/// controls.
#[test]
fn triangle_of_war_sacrifices_to_make_your_creature_fight_opponent_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), HILL_GIANT, GRIZZLY_BEARS],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[triangle_of_war()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hero = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let bystander = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let foe = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, triangle_of_war());
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, triangle_of_war(), 0);
    let own = aim_at(&mut engine, p0, hero);
    assert!(own.contains(&hero) && own.contains(&bystander));
    assert!(!own.contains(&foe), "the fighter is a creature you control");
    let theirs = aim_at(&mut engine, p0, foe);
    assert!(theirs.contains(&foe));
    assert!(
        !theirs.contains(&hero) && !theirs.contains(&bystander),
        "the foe is a creature an opponent controls"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, triangle_of_war()).is_some());
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().object(hero).unwrap().damage, 2);
    assert_eq!(engine.state().object(bystander).unwrap().damage, 0);
}
