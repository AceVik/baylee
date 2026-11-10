#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch46_pit_rejects_duplicate_targets_and_exiles_only_remaining_legal_targets() {
    let mut e = prepared(
        &[
            index::PIT_OF_OFFERINGS,
            index::LIGHTNING_BOLT,
            index::GIANT_GROWTH,
        ],
        &[],
        &[],
    );
    let bolt = hand_to_graveyard(&mut e, P0, index::LIGHTNING_BOLT);
    let growth = hand_to_graveyard(&mut e, P0, index::GIANT_GROWTH);
    let pit = play_land(&mut e, P0, index::PIT_OF_OFFERINGS);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt, bolt]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    choose(&mut e, vec![bolt, growth]);
    e.state
        .move_object(
            bolt,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bolt).unwrap().zone, Zone::Hand);
    assert_eq!(e.state.object(growth).unwrap().zone, Zone::Exile);
    pool(&mut e, &[]);
    e.state.set_tapped(pit, false);
    ability(&mut e, pit, 2);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 1);
}

#[test]
fn batch46_pit_links_colors_from_both_graveyards_and_forgets_departed_exile() {
    let mut e = prepared(
        &[
            index::PIT_OF_OFFERINGS,
            index::AMINATOU_THE_FATESHIFTER,
            index::LIGHTNING_BOLT,
        ],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let walker = hand_to_graveyard(&mut e, P0, index::AMINATOU_THE_FATESHIFTER);
    let bolt = hand_to_graveyard(&mut e, P0, index::LIGHTNING_BOLT);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    e.state
        .move_object(
            bear,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let pit = play_land(&mut e, P0, index::PIT_OF_OFFERINGS);
    assert!(is_tapped(&e, pit));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    choose(&mut e, vec![walker, bolt, bear]);
    pass_until(&mut e, stack_is_empty);
    for id in [walker, bolt, bear] {
        assert_eq!(e.state.object(id).unwrap().zone, Zone::Exile);
    }
    pool(&mut e, &[]);
    e.state.set_tapped(pit, false);
    ability(&mut e, pit, 2);
    assert_eq!(colors(&e), ManaColor::ALL[..5].to_vec());
    color(&mut e, ManaColor::Black);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Black), 1);

    e.state
        .move_object(
            walker,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            walker,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    // Returning to exile by another instruction must not restore the old link.
    e.state.set_tapped(pit, false);
    ability(&mut e, pit, 2);
    assert_eq!(colors(&e), vec![ManaColor::Red, ManaColor::Green]);
    color(&mut e, ManaColor::Green);
}

#[test]
fn batch46_pit_old_trigger_does_not_link_to_a_returned_incarnation() {
    let mut e = prepared(&[index::PIT_OF_OFFERINGS, index::LIGHTNING_BOLT], &[], &[]);
    let bolt = hand_to_graveyard(&mut e, P0, index::LIGHTNING_BOLT);
    let pit = play_land(&mut e, P0, index::PIT_OF_OFFERINGS);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    choose(&mut e, vec![bolt]);
    let old_version = e.state.object(pit).unwrap().version;
    // The exact public-zone moves of a blink, while the original ETB waits.
    e.state
        .move_object(
            pit,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            pit,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(e.state.object(pit).unwrap().version > old_version);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    choose(&mut e, vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bolt).unwrap().zone, Zone::Exile);
    pool(&mut e, &[]);
    e.state.set_tapped(pit, false);
    ability(&mut e, pit, 2);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
}

#[test]
fn batch46_pit_without_a_colored_link_keeps_only_its_colorless_route() {
    for exile_colorless in [false, true] {
        let mut e = prepared(&[index::PIT_OF_OFFERINGS, index::FOREST], &[], &[]);
        let forest = hand_to_graveyard(&mut e, P0, index::FOREST);
        let pit = play_land(&mut e, P0, index::PIT_OF_OFFERINGS);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        choose(
            &mut e,
            if exile_colorless {
                vec![forest]
            } else {
                vec![]
            },
        );
        pass_until(&mut e, stack_is_empty);
        pool(&mut e, &[]);
        e.state.set_tapped(pit, false);
        ability(&mut e, pit, 2);
        assert_eq!(e.state.players[0].mana_pool.total(), 0);
        e.state.set_tapped(pit, false);
        ability(&mut e, pit, 1);
        assert_eq!(
            e.state.players[0].mana_pool.available(ManaColor::Colorless),
            1
        );
    }
}
