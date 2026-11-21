//! `cards/instants/mv_2/purge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d06f0f7c-0270-44be-991a-a5ebbddb0900"
fn purge() -> CardIndex {
    card_index("d06f0f7c-0270-44be-991a-a5ebbddb0900")
}

/// Purge ({1}{W}): targets ONLY artifact creatures or black creatures
/// (excluding noncreature artifacts and nonblack nonartifact creatures) and
/// destroys them through an active regeneration shield.
#[test]
fn purge_targets_only_artifact_or_black_creature_and_ignores_regeneration() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[purge(), purge()])
        .battlefield(
            1,
            &[
                swamp(),
                index::DRUDGE_SKELETONS,
                index::ORNITHOPTER,
                sol_ring(),
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let skel = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);
    let thopter = lk::obj(&engine, P1, index::ORNITHOPTER);
    let ring = lk::obj(&engine, P1, sol_ring());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    let ps = all_on_battlefield(&engine, P0, plains());
    tap_mana_where(&mut engine, P0, |id| ps[..2].contains(&id));
    cast_with_floating(&mut engine, P0, purge());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&skel) && options.contains(&thopter));
    assert!(
        !options.contains(&ring) && !options.contains(&bears),
        "noncreature artifact and green creature excluded: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![skel],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 regenerates Drudge Skeletons in response -> Purge ignores shield.
    tap_all_mana(&mut engine, P1);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: skel,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(skel)
            .is_some_and(|o| o.regeneration_shields == 1)
    });
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P1, index::DRUDGE_SKELETONS).is_some());

    // Second Purge destroys the artifact creature Ornithopter.
    tap_mana_where(&mut engine, P0, |id| ps[2..].contains(&id));
    lk::cast_at(&mut engine, P0, purge(), &[thopter], &[]);
    assert!(in_graveyard(&engine, P1, index::ORNITHOPTER).is_some());
}
