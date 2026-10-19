//! `cards/creatures/mv_3/grave_robbers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grave_robbers() -> CardIndex {
    card_index("6698b059-f1ed-4023-9d44-fbdf39fb159a")
}

fn float(engine: &mut Engine<RegistryLookup>, color: ManaColor, n: u32) {
    engine.state.players[0].mana_pool.add(color, n);
    engine.refresh_offer();
}

fn graveyard(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .clone()
}

fn exile(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Exile(seat)).clone()
}

/// Grave Robbers (6698b059-f1ed-4023-9d44-fbdf39fb159a) — "{B}, {T}: Exile
/// target artifact card from a graveyard. You gain 2 life." Artifact cards in
/// either graveyard are targets; a creature card is not.
#[test]
fn grave_robbers_exiles_an_artifact_card_from_a_graveyard_and_gains_two() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[grave_robbers(), quiet_artifact()])
        .battlefield(1, &[quiet_artifact(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let robbers = on_battlefield(&engine, P0, grave_robbers()).expect("the Robbers are out");
    let mine = on_battlefield(&engine, P0, quiet_artifact()).expect("my artifact");
    let theirs = on_battlefield(&engine, P1, quiet_artifact()).expect("their artifact");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their creature");
    bury(&mut engine, &[mine, theirs, ogre]);
    engine.refresh_offer();
    let my_ring = in_graveyard(&engine, P0, quiet_artifact()).expect("my artifact card is there");
    let their_ring =
        in_graveyard(&engine, P1, quiet_artifact()).expect("their artifact card is there");
    let their_ogre = in_graveyard(&engine, P1, gray_ogre()).expect("their creature card");

    float(&mut engine, ManaColor::Black, 1);
    activate(&mut engine, P0, grave_robbers(), 0);
    let menu = aim_at(&mut engine, P0, their_ring);
    assert!(menu.contains(&my_ring), "my graveyard's artifact card too");
    assert!(menu.contains(&their_ring));
    assert!(
        !menu.contains(&their_ogre),
        "a creature card is not an artifact card"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, robbers), "{{T}} was paid");
    assert_eq!(engine.state().players[0].life, 22, "gained 2");
    assert_eq!(
        exile(&engine, P1).len(),
        1,
        "their artifact card was exiled"
    );
    assert!(
        in_graveyard(&engine, P1, quiet_artifact()).is_none(),
        "and left the graveyard"
    );
    assert!(
        in_graveyard(&engine, P0, quiet_artifact()).is_some(),
        "my own artifact card was not touched"
    );
    assert!(
        in_graveyard(&engine, P1, gray_ogre()).is_some(),
        "the creature card stayed"
    );
    assert_eq!(graveyard(&engine, P1).len(), 1);
}

/// No artifact card in any graveyard: no target, no activation.
#[test]
fn grave_robbers_needs_an_artifact_card_in_a_graveyard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[grave_robbers(), quiet_artifact()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let robbers = on_battlefield(&engine, P0, grave_robbers()).expect("the Robbers are out");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their creature");
    bury(&mut engine, &[ogre]);
    engine.refresh_offer();
    float(&mut engine, ManaColor::Black, 1);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: robbers,
                    ability_index: 0
                }
            )
            .is_err(),
        "the artifact is on the battlefield and the graveyard holds only a creature"
    );
    assert!(!is_tapped(&engine, robbers));
    assert_eq!(engine.state().players[0].life, 20);
}
