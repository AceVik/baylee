//! `cards/artifacts/mv_3/skeleton_shard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn skeleton_shard() -> CardIndex {
    card_index("076fa82d-2a62-4590-bff7-69f0d3bee647")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Skeleton Shard (076fa82d-2a62-4590-bff7-69f0d3bee647) — {3} Artifact:
/// "{3}, {T} or {B}, {T}: Return target artifact creature card from your
///  graveyard to your hand."
///
/// Exercises both activation costs ({3}, {T} and {B}, {T}) and verifies the
/// target filter admits only artifact creature cards in your own graveyard.
#[test]
fn skeleton_shard_returns_own_artifact_creature_via_either_cost() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                skeleton_shard(),
                steel_wall(),
                ornithopter(),
                grizzly_bears(),
                sol_ring(),
                forest(),
                forest(),
                forest(),
                swamp(),
            ],
        )
        .battlefield(1, &[steel_wall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_wall = on_battlefield(&engine, p0, steel_wall()).unwrap();
    let my_thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let my_ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let their_wall = on_battlefield(&engine, p1, steel_wall()).unwrap();
    bury(
        &mut engine,
        &[my_wall, my_thopter, my_bears, my_ring, their_wall],
    );
    engine.refresh_offer();

    // 1. Ability 0 ({3}, {T}) using 3 Forests (no black mana floating).
    let my_forests = forests(&engine, p0, 3);
    tap_mana_where(&mut engine, p0, |id| my_forests.contains(&id));
    let shard = on_battlefield(&engine, p0, skeleton_shard()).unwrap();
    let offer = priority_offer(&engine);
    assert!(
        offer.abilities.contains(&(shard, 0)) && !offer.abilities.contains(&(shard, 1)),
        "3 green mana floating pays {{3}}, {{T}} but not {{B}}, {{T}}"
    );

    activate(&mut engine, p0, skeleton_shard(), 0);
    let menu = aim_at(&mut engine, p0, my_wall);
    assert!(
        menu.contains(&my_wall) && menu.contains(&my_thopter),
        "own artifact creature cards are offered: {menu:?}"
    );
    assert!(
        !menu.contains(&my_bears) && !menu.contains(&my_ring) && !menu.contains(&their_wall),
        "non-artifact creature, noncreature artifact, and opponent's graveyard excluded: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(my_wall).unwrap().zone, Zone::Hand);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // 2. Next turn: Ability 1 ({B}, {T}) using 1 Swamp.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);

    let my_swamp = on_battlefield(&engine, p0, swamp()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_swamp);
    let offer = priority_offer(&engine);
    assert!(
        !offer.abilities.contains(&(shard, 0)) && offer.abilities.contains(&(shard, 1)),
        "1 black mana floating pays {{B}}, {{T}} but not {{3}}, {{T}}"
    );

    activate(&mut engine, p0, skeleton_shard(), 1);
    aim_at(&mut engine, p0, my_thopter);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(my_thopter).unwrap().zone, Zone::Hand);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
