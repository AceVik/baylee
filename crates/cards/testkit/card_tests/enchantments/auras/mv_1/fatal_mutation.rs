//! `cards/enchantments/auras/mv_1/fatal_mutation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn fatal_mutation() -> CardIndex {
    card_index("26d3077f-ad1f-47fb-a96d-8aac4bdef8a3")
}

/// Fatal Mutation enchants a face-down creature without destroying it, destroys it
/// without regeneration when turned face up, and ignores unenchanted creatures
/// turning face up.
#[test]
fn fatal_mutation_destroys_enchanted_creature_no_regen_and_ignores_unenchanted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(
            0,
            &[
                index::SKIRK_COMMANDO,
                index::SKIRK_COMMANDO,
                fatal_mutation(),
                index::REGENERATE,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mtns = all_on_battlefield(&engine, p0, mountain());
    for chunk in mtns.as_chunks::<3>().0 {
        tap_mana_where(&mut engine, p0, |id| chunk.contains(&id));
        cast_with_floating(&mut engine, p0, index::SKIRK_COMMANDO);
        let slot = choose_cast_kind(&engine, CastModeKind::Disguise);
        engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
        pass_until(&mut engine, |e| at_rest(e, p0));
    }

    let face_downs: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.status.contains(Status::FACE_DOWN))
        })
        .collect();
    assert_eq!(face_downs.len(), 2);
    let (enchanted_fd, unenchanted_fd) = (face_downs[0], face_downs[1]);

    let sw = on_battlefield(&engine, p0, swamp()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == sw);
    cast_with_floating(&mut engine, p0, fatal_mutation());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![enchanted_fd],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&enchanted_fd)
    );

    // Untap on P0's next main phase.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));

    // Give the enchanted face-down creature a regeneration shield before turning it face up.
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    cast_with_floating(&mut engine, p0, index::REGENERATE);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![enchanted_fd],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let mtns = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mtns[..3].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: enchanted_fd,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, index::SKIRK_COMMANDO).is_some(),
        "enchanted creature was destroyed through its regeneration shield"
    );
    assert!(in_graveyard(&engine, p0, fatal_mutation()).is_some());

    // Turning the unenchanted face-down creature face up does not trigger Fatal Mutation.
    tap_mana_where(&mut engine, p0, |id| mtns[3..6].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: unenchanted_fd,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&unenchanted_fd),
        "unenchanted face-down creature survives turning face up"
    );
}
