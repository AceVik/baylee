//! `cards/instants/mv_4/death_bomb.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e323d468-def5-4090-af5b-19481289168f"
fn death_bomb() -> CardIndex {
    card_index("e323d468-def5-4090-af5b-19481289168f")
}

/// Death Bomb ({3}{B}): cannot be cast without a creature you control to
/// sacrifice.
#[test]
fn death_bomb_requires_a_controlled_creature_to_sacrifice() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[death_bomb()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 4)]);
    assert!(
        !lk::castable(&engine, P0, death_bomb()),
        "cannot sacrifice opponent's creature"
    );
}

/// Sacrificing a creature destroys target nonblack creature (rejecting black
/// creatures and ignoring regeneration shields) and makes its controller lose
/// 2 life.
#[test]
fn death_bomb_sacrifices_ignores_regeneration_and_costs_controller_two_life() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), index::LLANOWAR_ELVES],
        )
        .hand(0, &[death_bomb()])
        .battlefield(1, &[forest(), index::RIVER_BOA, index::DRUDGE_SKELETONS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let boa = lk::obj(&engine, P1, index::RIVER_BOA);
    let skel = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);

    let swamps = all_on_battlefield(&engine, P0, swamp());
    tap_mana_where(&mut engine, P0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, P0, death_bomb());
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                assert!(options.contains(&boa));
                assert!(
                    !options.contains(&skel),
                    "black creature is excluded: {options:?}"
                );
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![boa],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert!(in_graveyard(&engine, P0, index::LLANOWAR_ELVES).is_some());
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 activates River Boa's regeneration shield in response.
    tap_all_mana(&mut engine, P1);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: boa,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(boa)
            .is_some_and(|o| o.regeneration_shields == 1)
    });

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P1, index::RIVER_BOA).is_some(),
        "destroyed through active regeneration shield"
    );
    assert_eq!(lk::life(&engine, P1), 18, "target's controller lost 2 life");
}
