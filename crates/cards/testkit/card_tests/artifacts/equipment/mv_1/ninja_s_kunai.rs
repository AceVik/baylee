//! `cards/artifacts/equipment/mv_1/ninja_s_kunai.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ninja_s_kunai() -> CardIndex {
    card_index("fa5aefaf-432f-41d0-8e54-0e8b7bc7ba98")
}

fn walking_corpse() -> CardIndex {
    card_index("fea95888-e16a-4209-9cd4-623f7f4d2f67")
}

/// Equip {1} moves the granted ability to the new wearer. Its additional {1}
/// and {T} are real costs, the Kunai rather than either creature is sacrificed,
/// and the sacrificed Kunai is the source of the three damage to any target.
#[test]
fn kunai_moves_its_shot_then_pays_and_deals_three_as_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = rig(
        3,
        &[],
        &[ninja_s_kunai(), tragic_poet(), tragic_poet()],
        &[walking_corpse()],
    );
    let poets = all_on_battlefield(&engine, p0, tragic_poet());
    let (first, second) = (poets[0], poets[1]);
    let granted = baylee_engine::choice::granted_ability(0);
    let offered =
        |e: &Engine<RegistryLookup>, who| priority_offer(e).abilities.contains(&(who, granted));
    let kunai = equip(&mut engine, ninja_s_kunai(), 1, first);
    assert!(
        !offered(&engine, first),
        "the granted shot still needs its own {{1}} after Equip"
    );
    assert!(!offered(&engine, second));

    equip(&mut engine, ninja_s_kunai(), 1, second);
    assert!(!offered(&engine, first), "the old wearer loses the shot");
    assert!(
        !offered(&engine, second),
        "the additional {{1}} is not free"
    );
    float(&mut engine, 1);
    assert!(offered(&engine, second));
    let corpse = on_battlefield(&engine, p1, walking_corpse()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: second,
                ability_index: granted,
            },
        )
        .expect("the equipped creature activates the shot");
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the shot asks for any target: {:?}", engine.pending())
    };
    assert!(options.contains(&corpse));
    assert!(player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    assert!(is_tapped(&engine, second), "the wearer paid {{T}}");
    assert!(!is_tapped(&engine, first));
    assert_eq!(pool_total(&engine), 0, "the additional {{1}} was paid");
    assert_eq!(engine.state().object(kunai).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(second).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(first).unwrap().zone,
        Zone::Battlefield
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 17, "the Kunai dealt three");
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::DamageDealt {
                    source: Some(source),
                    target: baylee_engine::event::DamageTarget::Player(player),
                    amount: 3,
                    is_combat: false,
                } if source == kunai && player == p1
            )),
        "the sacrificed Kunai, not the wearer, dealt the three"
    );
}
