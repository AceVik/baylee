//! `cards/instants/mv_1/disarm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn disarm() -> CardIndex {
    card_index("44d54814-2eeb-4df8-b8d3-789f0bcb307a")
}

fn dead_iron_sledge() -> CardIndex {
    card_index("e1ee79de-e4c3-48f9-87a7-18f72e2f6116")
}

/// Disarm — {U} Instant
/// "Unattach all Equipment from target creature."
#[test]
fn disarm_unattaches_all_equipment_from_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[disarm()])
        .battlefield(
            0,
            &[
                dead_iron_sledge(),
                grizzly_bears(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sledge = on_battlefield(&engine, p0, dead_iron_sledge()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let islands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().card.unwrap().index == island())
        .collect();

    tap_mana_where(&mut engine, p0, |id| islands[..2].contains(&id));
    activate(&mut engine, p0, dead_iron_sledge(), 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(sledge).unwrap().attached_to,
        Some(bear)
    );

    cast_from_hand(&mut engine, p0, disarm());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(sledge).unwrap().attached_to, None);
    assert!(on_battlefield(&engine, p0, dead_iron_sledge()).is_some());
}
