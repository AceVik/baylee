//! `cards/lands/utility/unstable_frontier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unstable Frontier: "{T}: Add {C}." / "{T}: Target land you control becomes the basic land type of your choice until end of turn."
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn unstable_frontier_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest())
        .battlefield(0, &[unstable_frontier()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let frontier = on_battlefield(&engine, p0, unstable_frontier()).expect("Frontier deployed");
    activate(&mut engine, p0, unstable_frontier(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, frontier));
}

use baylee_core::generated::{index, subtypes};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

#[test]
fn unstable_frontier_chooses_each_basic_type_only_when_resolving() {
    let types = [
        subtypes::land::PLAINS,
        subtypes::land::ISLAND,
        subtypes::land::SWAMP,
        subtypes::land::MOUNTAIN,
        subtypes::land::FOREST,
    ];
    for (mode, chosen) in types.into_iter().enumerate() {
        let mut e = prepared(&[], &[index::UNSTABLE_FRONTIER, index::ISLAND], &[]);
        let island = on_battlefield(&e, P0, index::ISLAND).unwrap();
        activate(&mut e, P0, index::UNSTABLE_FRONTIER, 1);
        assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
        aim_at(&mut e, P0, island);
        assert!(matches!(e.pending(), Pending::Priority { .. }));
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let c = e.state.object(island).unwrap().characteristics();
        assert!(c.types.contains(baylee_core::types::TypeSet::LAND));
        assert!(
            c.supertypes
                .contains(baylee_core::types::SupertypeSet::BASIC)
        );
        for t in types {
            assert_eq!(c.subtypes.contains(t), t == chosen);
        }
        pass_until(&mut e, |e| {
            e.state.turn.active == P1
                && matches!(e.state.turn.phase, Phase::FirstMain)
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
        });
        let c = e.state.object(island).unwrap().characteristics();
        assert!(c.subtypes.contains(subtypes::land::ISLAND));
        for t in types {
            assert_eq!(c.subtypes.contains(t), t == subtypes::land::ISLAND);
        }
    }
}

#[test]
fn unstable_frontier_can_target_itself_and_loses_its_printed_abilities_until_cleanup() {
    let mut e = prepared(&[], &[index::UNSTABLE_FRONTIER, index::FOREST], &[]);
    let frontier = on_battlefield(&e, P0, index::UNSTABLE_FRONTIER).unwrap();
    activate(&mut e, P0, index::UNSTABLE_FRONTIER, 1);
    aim_at(&mut e, P0, frontier);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(P0, PlayerAction::ChooseMode(4)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state
            .object(frontier)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST)
    );
    e.state.set_tapped(frontier, false); // isolate type-derived ability from the tap cost
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, index)| *source == frontier && *index == 1)
    );
    let green = e.state.players[0].mana_pool.available(ManaColor::Green);
    e.apply(P0, PlayerAction::ActivateManaAbility { source: frontier })
        .unwrap();
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Green),
        green + 1
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: frontier,
                ability_index: 1
            }
        )
        .is_err()
    );
    pass_until(&mut e, |e| {
        e.state.turn.active == P1
            && matches!(e.state.turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    assert!(
        !e.state
            .object(frontier)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST)
    );
}

#[test]
fn unstable_frontier_targets_only_own_lands_and_still_has_colorless_mana() {
    let mut e = prepared(
        &[],
        &[index::UNSTABLE_FRONTIER, index::GRIZZLY_BEARS],
        &[index::ISLAND],
    );
    let frontier = on_battlefield(&e, P0, index::UNSTABLE_FRONTIER).unwrap();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let foreign = on_battlefield(&e, P1, index::ISLAND).unwrap();
    activate(&mut e, P0, index::UNSTABLE_FRONTIER, 1);
    for illegal in [bear, foreign] {
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![illegal],
                    players: vec![]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
    }
    aim_at(&mut e, P0, frontier);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let mut mana = prepared(&[], &[index::UNSTABLE_FRONTIER, index::FOREST], &[]);
    let before = mana.state.players[0]
        .mana_pool
        .available(ManaColor::Colorless);
    activate(&mut mana, P0, index::UNSTABLE_FRONTIER, 0);
    assert_eq!(
        mana.state.players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        before + 1
    );
}
