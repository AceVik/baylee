//! `cards/creatures/mv_3/zombie_master.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zombie Master — "Other Zombie creatures have swampwalk." / "Other
/// Zombies have '{B}: Regenerate this permanent.'" Neither line reaches the
/// master itself.
#[test]
fn zombie_master_grants_other_zombies_swampwalk_and_a_regenerate_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[zombie_master(), scathe_zombies(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let master = on_battlefield(&engine, p0, zombie_master()).expect("seated");
    let zombie = on_battlefield(&engine, p0, scathe_zombies()).expect("another Zombie");

    assert!(
        !keywords(&engine, master).contains(KeywordSet::SWAMPWALK),
        "\"other\" Zombies, not itself"
    );
    assert!(keywords(&engine, zombie).contains(KeywordSet::SWAMPWALK));

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|&(id, _)| id == master),
        "the master itself is granted nothing: {:?}",
        legal.abilities
    );
    assert!(
        legal
            .abilities
            .contains(&(zombie, crate::choice::GRANTED_ABILITY)),
        "the granted regeneration is offered on the other Zombie: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: zombie,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .expect("the floating {B} pays for the grant");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(zombie).unwrap().regeneration_shields,
        1,
        "the granted ability regenerates the Zombie it was granted to"
    );
}

/// "Other Zombies" are not "your Zombies". With the Zombie Master on our side
/// the *opponent's* Zombie has swampwalk and is offered the regeneration to
/// its own controller; their non-Zombie gets neither.
#[test]
fn zombie_master_grants_the_opponents_zombies_too_and_their_controller_may_use_it() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[zombie_master()])
        .battlefield(1, &[scathe_zombies(), quiet_creature(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let zombie = on_battlefield(&engine, p1, scathe_zombies()).expect("their Zombie");
    let bystander = on_battlefield(&engine, p1, quiet_creature()).expect("their non-Zombie");
    assert!(
        keywords(&engine, zombie).contains(KeywordSet::SWAMPWALK),
        "the opponent's Zombie has swampwalk from our master"
    );
    assert!(!keywords(&engine, bystander).contains(KeywordSet::SWAMPWALK));

    tap_all_mana(&mut engine, p1);
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert!(
        legal
            .abilities
            .contains(&(zombie, crate::choice::GRANTED_ABILITY)),
        "the regeneration is offered to the Zombie's own controller: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.iter().any(|&(id, _)| id == bystander),
        "a non-Zombie is granted nothing: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: zombie,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .expect("the opponent's floating {B} pays for the grant");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(zombie).unwrap().regeneration_shields,
        1,
        "their Zombie regenerates"
    );
}
