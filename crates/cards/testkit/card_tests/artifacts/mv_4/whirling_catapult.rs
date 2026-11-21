//! `cards/artifacts/mv_4/whirling_catapult.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn whirling_catapult() -> CardIndex {
    card_index("0987b5e9-0012-4189-8745-45f10e9557f3")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Whirling Catapult (0987b5e9-0012-4189-8745-45f10e9557f3) — {4} Artifact:
/// "{2}, Exile the top two cards of your library: This artifact deals 1 damage
///  to each creature with flying and each player."
///
/// Exiles the top 2 cards of your library as part of the activation cost,
/// deals 1 damage to each flyer on both sides and both players, and spares
/// non-flying creatures.
#[test]
fn whirling_catapult_exiles_top_two_and_hits_flyers_and_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                whirling_catapult(),
                serra_angel(),
                grizzly_bears(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[air_elemental(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elemental = on_battlefield(&engine, p1, air_elemental()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    let lib_before = library_size(&engine, p0);
    let top_two: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .iter()
        .rev()
        .take(2)
        .copied()
        .collect();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, whirling_catapult(), 0);

    // Cost paid up front: top two cards of library moved to exile.
    assert_eq!(library_size(&engine, p0), lib_before - 2);
    let exile = engine.state().zones.list(ZoneLocation::Exile(p0));
    assert_eq!(exile.len(), 2);
    for id in &top_two {
        assert!(exile.contains(id), "top library card {id:?} was exiled");
    }

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        1,
        "own creature with flying took 1 damage"
    );
    assert_eq!(
        engine.state().object(elemental).unwrap().damage,
        1,
        "opponent's creature with flying took 1 damage"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        0,
        "non-flying Grizzly Bears took 0 damage"
    );
    assert_eq!(
        engine.state().object(elves).unwrap().zone,
        Zone::Battlefield,
        "non-flying 1/1 Llanowar Elves survived with 0 damage"
    );
    assert_eq!(engine.state().object(elves).unwrap().damage, 0);
    assert_eq!(life_of(&engine, p0), 19, "controller took 1 damage");
    assert_eq!(life_of(&engine, p1), 19, "opponent took 1 damage");
}

/// Cannot activate Whirling Catapult when your library has fewer than two cards.
#[test]
fn whirling_catapult_refused_with_fewer_than_two_cards_in_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[whirling_catapult(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Leave only 1 card in p0's library so ExileTopOfLibrary(2) is unpayable.
    let to_exile: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .iter()
        .skip(1)
        .copied()
        .collect();
    let state = engine.dev_state_mut(p0).unwrap();
    for id in to_exile {
        state
            .move_object(
                id,
                ZoneLocation::Exile(p0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
    assert_eq!(library_size(&engine, p0), 1);

    let catapult = on_battlefield(&engine, p0, whirling_catapult()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(
        !ability_offered(&engine, catapult, 0),
        "cannot pay 'Exile the top two cards of your library' with only 1 card"
    );
}
