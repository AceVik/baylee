//! `cards/creatures/mv_4/sand_squid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sand_squid() -> CardIndex {
    card_index("40578245-2cb8-4a6a-aeff-2252719c2d1c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

/// Sand Squid (40578245-2cb8-4a6a-aeff-2252719c2d1c) — `{3}{U}` 2/2 Creature — Squid Beast:
/// - Islandwalk
/// - You may choose not to untap this creature during your untap step.
/// - `{T}`: Tap target creature. That creature doesn't untap during its controller's
///   untap step for as long as this creature remains tapped.
#[test]
fn sand_squid_casts_and_has_islandwalk() {
    let mut with_island = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears(), island()])
        .hand(0, &[sand_squid()])
        .start();
    keep_mulligans(&mut with_island);
    reach_main_phase(&mut with_island, P0);

    cast_from_hand(&mut with_island, P0, sand_squid());
    pass_until(&mut with_island, stack_is_empty);

    let squid = on_battlefield(&with_island, P0, sand_squid()).expect("Sand Squid entered");
    assert_eq!(pt(&with_island, squid), (2, 2));
    assert!(keywords(&with_island, squid).contains(KeywordSet::ISLANDWALK));

    reach_their_main_phase(&mut with_island, P1);
    reach_their_main_phase(&mut with_island, P0);
    let blocks = attack_and_collect_blocks(&mut with_island, squid, P1);
    assert!(
        blocks.is_empty(),
        "islandwalk makes Sand Squid unblockable while P1 controls an Island"
    );

    let mut without_island = Duel::new(SEED, island())
        .battlefield(0, &[sand_squid()])
        .battlefield(1, &[grizzly_bears(), forest()])
        .start();
    keep_mulligans(&mut without_island);
    let squid2 = on_battlefield(&without_island, P0, sand_squid()).unwrap();
    let blocks_no_island = attack_and_collect_blocks(&mut without_island, squid2, P1);
    assert_eq!(
        blocks_no_island.len(),
        1,
        "without an Island on P1's board, Grizzly Bears can block Sand Squid"
    );
}

#[test]
fn sand_squid_locks_target_tapped_while_kept_tapped_and_releases_when_untapped() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[sand_squid(), island()])
        .battlefield(1, &[grizzly_bears(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let squid = on_battlefield(&engine, P0, sand_squid()).unwrap();
    let p0_island = on_battlefield(&engine, P0, island()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let p1_forest = on_battlefield(&engine, P1, forest()).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, sand_squid(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, squid) && is_tapped(&engine, bears));

    // P1's untap step: Sand Squid is tapped, so Bears stays tapped while P1's Forest untaps.
    tap_all_mana(&mut engine, P1);
    reach_their_main_phase(&mut engine, P1);
    assert!(
        is_tapped(&engine, bears),
        "target creature does not untap while Sand Squid remains tapped"
    );
    assert!(!is_tapped(&engine, p1_forest));

    // P0's untap step: choose to leave Sand Squid tapped; P0's Island still untaps.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![squid],
            },
        )
        .expect("P0 chooses not to untap Sand Squid");
    reach_main_phase(&mut engine, P0);
    assert!(is_tapped(&engine, squid));
    assert!(!is_tapped(&engine, p0_island));

    // P1's next untap step: Bears is still locked down.
    reach_their_main_phase(&mut engine, P1);
    assert!(is_tapped(&engine, bears));

    // P0's next untap step: choose empty set so Sand Squid untaps, releasing Bears.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("P0 lets Sand Squid untap");
    reach_main_phase(&mut engine, P0);
    assert!(!is_tapped(&engine, squid));

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !is_tapped(&engine, bears),
        "once Sand Squid untaps, target creature untaps on its controller's untap step"
    );
}
