//! Chandler — `{4}{R}` 3/3 Legendary Creature — Human Rogue (Homelands)
//!
//! `{R}{R}{R}`, `{T}`: Destroy target artifact creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn chandler() -> CardIndex {
    card_index("520abf49-7631-4e7f-889e-547ccc8c3b95")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

#[test]
fn chandler_destroys_target_artifact_creature_and_cannot_target_nonartifact_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[chandler(), mountain(), mountain(), mountain()])
        .battlefield(1, &[obsianus_golem(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let chandler_id = on_battlefield(&engine, p0, chandler()).unwrap();
    let golem = on_battlefield(&engine, p1, obsianus_golem()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, chandler_id), (3, 3));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: chandler_id,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&golem),
        "artifact creature is a legal target"
    );
    assert!(
        !options.contains(&bears),
        "nonartifact creature is not a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![golem],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(golem).unwrap().zone,
        Zone::Graveyard,
        "Obsianus Golem is destroyed by Chandler"
    );
    assert!(is_tapped(&engine, chandler_id));
}
