//! Bosh, Iron Golem — `{8}` 6/7 Legendary Artifact Creature — Golem (MRD).
//! Oracle: "Trample. {3}{R}, Sacrifice an artifact: Bosh deals damage equal to the sacrificed artifact's mana value to any target."

use super::*;

fn bosh_iron_golem() -> CardIndex {
    card_index("2fb6f65f-a0fb-4d45-be1b-d6405e61d47a")
}

#[test]
fn bosh_iron_golem_sacrifices_an_artifact_and_deals_damage_equal_to_its_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let bosh = bosh_iron_golem();
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[bosh, mountain(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bosh_id = on_battlefield(&engine, p0, bosh).unwrap();
    assert!(keywords(&engine, bosh_id).contains(KeywordSet::TRAMPLE));
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, bosh, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bosh_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, bosh).is_none());
    assert!(in_graveyard(&engine, p0, bosh).is_some());
    assert_eq!(engine.state().players[1].life, 12);
}
