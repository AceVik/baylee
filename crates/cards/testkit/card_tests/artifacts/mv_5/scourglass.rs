//! `cards/artifacts/mv_5/scourglass.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scourglass() -> CardIndex {
    card_index("78541a40-c27a-49c0-aa3e-5a48b97065f0")
}

fn crusade() -> CardIndex {
    card_index("4692740f-be90-459f-8d90-c4ae71771595")
}

fn elspeth_storm_slayer() -> CardIndex {
    card_index("f78af825-023a-42e9-8374-5c52303a1417")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Scourglass (78541a40-c27a-49c0-aa3e-5a48b97065f0) — {3}{W}{W} Artifact:
/// "{T}, Sacrifice this artifact: Destroy all permanents except for artifacts
/// and lands. Activate only during your upkeep."
///
/// Not activatable in your main phase or during opponent's upkeep; during your
/// upkeep destroys creatures, enchantments, and planeswalkers on both sides
/// while sparing artifacts, artifact creatures, and lands.
#[test]
fn scourglass_upkeep_only_and_destroys_all_nonartifact_nonland_permanents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                scourglass(),
                grizzly_bears(),
                crusade(),
                ornithopter(),
                sol_ring(),
                plains(),
            ],
        )
        .battlefield(1, &[hill_giant(), elspeth_storm_slayer(), forest()])
        .start();
    keep_mulligans(&mut engine);

    // During your main phase (not upkeep): NOT legal.
    reach_main_phase(&mut engine, p0);
    let glass = on_battlefield(&engine, p0, scourglass()).unwrap();
    assert!(
        !priority_offer(&engine).abilities.contains(&(glass, 0)),
        "Scourglass cannot be activated during your main phase"
    );

    // During opponent's upkeep: NOT legal.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        !priority_offer(&engine).abilities.contains(&(glass, 0)),
        "Scourglass cannot be activated during opponent's upkeep"
    );

    // Advance to p0's next upkeep: legal -> activate!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        priority_offer(&engine).abilities.contains(&(glass, 0)),
        "Scourglass IS legal during your upkeep"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: glass,
                ability_index: 0,
            },
        )
        .expect("activate Scourglass during your upkeep");
    pass_until(&mut engine, stack_is_empty);

    // Destroyed: Scourglass (sacrificed), Grizzly Bears, Crusade, Hill Giant, Karn.
    assert!(in_graveyard(&engine, p0, scourglass()).is_some());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, crusade()).is_some());
    assert!(in_graveyard(&engine, p1, hill_giant()).is_some());
    assert!(in_graveyard(&engine, p1, elspeth_storm_slayer()).is_some());

    // Spared: artifact creature (Ornithopter), noncreature artifact (Sol Ring), lands.
    assert!(
        on_battlefield(&engine, p0, ornithopter()).is_some(),
        "artifact creature is an artifact and survives Scourglass"
    );
    assert!(on_battlefield(&engine, p0, sol_ring()).is_some());
    assert!(on_battlefield(&engine, p0, plains()).is_some());
    assert!(on_battlefield(&engine, p1, forest()).is_some());
}
