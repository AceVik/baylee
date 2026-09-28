use super::{testkit::*, *};
use crate::{
    event::Cause,
    zone::{ZoneLocation, ZonePosition},
};
use baylee_core::ids::CardIndex;
const P0: PlayerId = PlayerId::new(0);
fn card(name: &str) -> CardIndex {
    baylee_cards::generated::ALL
        .iter()
        .find(|(_, c)| c.name() == name)
        .unwrap_or_else(|| panic!("missing card {name}"))
        .1
        .index
}
fn move_to_graveyard(e: &mut Engine<RegistryLookup>, id: ObjectId, player: PlayerId) {
    e.state
        .move_object(
            id,
            ZoneLocation::Graveyard(player),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
}
fn setup() -> Engine<RegistryLookup> {
    let mut e = Duel::table(9102, card("Forest"), 3)
        .battlefield(
            0,
            &[
                card("Krark-Clan Ironworks"),
                card("Mind Stone"),
                card("Forest"),
                card("Forest"),
                card("Forest"),
            ],
        )
        .hand(
            0,
            &[
                card("Scrap Trawler"),
                card("Sol Ring"),
                card("Darksteel Pendant"),
                card("Llanowar Elves"),
                card("Lotus Petal"),
            ],
        )
        .hand(1, &[card("Sol Ring")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for name in [
        "Sol Ring",
        "Darksteel Pendant",
        "Llanowar Elves",
        "Lotus Petal",
    ] {
        let id = in_hand(&e, P0, card(name)).unwrap();
        move_to_graveyard(&mut e, id, P0);
    }
    let p1 = PlayerId::new(1);
    let id = in_hand(&e, p1, card("Sol Ring")).unwrap();
    move_to_graveyard(&mut e, id, p1);
    cast_from_hand(&mut e, P0, card("Scrap Trawler"));
    pass_until(&mut e, stack_is_empty);
    e
}
fn sacrifice(e: &mut Engine<RegistryLookup>, name: &str) -> ObjectId {
    let source = on_battlefield(e, P0, card("Krark-Clan Ironworks")).unwrap();
    let id = on_battlefield(e, P0, card(name)).unwrap();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![id] })
        .unwrap();
    id
}
fn targets(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    match e.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("expected targets, got {other:?}"),
    }
}
#[test]
fn scrap_trawler_uses_the_dead_artifacts_value_not_its_own() {
    let mut e = setup();
    let stone = sacrifice(&mut e, "Mind Stone");
    let options = targets(&e);
    let ring = in_graveyard(&e, P0, card("Sol Ring")).unwrap();
    let mox = in_graveyard(&e, P0, card("Lotus Petal")).unwrap();
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ring) && options.contains(&mox));
    assert!(!options.contains(&stone));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![stone]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![ring],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, card("Sol Ring")).is_some());
    assert!(
        in_graveyard(&e, P0, card("Darksteel Pendant")).is_some(),
        "equal mana value stays"
    );
}
#[test]
fn scrap_trawler_own_death_has_its_own_bound_and_zero_has_no_targets() {
    let mut e = setup();
    sacrifice(&mut e, "Scrap Trawler");
    let options = targets(&e);
    let pendant = in_graveyard(&e, P0, card("Darksteel Pendant")).unwrap();
    assert_eq!(options.len(), 3);
    assert!(options.contains(&pendant));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![pendant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, card("Darksteel Pendant")).is_some());

    let mut zero = setup();
    let stone = on_battlefield(&zero, P0, card("Mind Stone")).unwrap();
    // A zero-mana artifact fixture; the trigger is still created through an actual sacrifice.
    std::sync::Arc::make_mut(&mut zero.state.object_mut(stone).unwrap().base).mana_cost =
        baylee_core::mana::ManaCost::ZERO;
    sacrifice(&mut zero, "Mind Stone");
    assert!(matches!(zero.pending(), Pending::Priority { .. }));
    assert!(stack_is_empty(&zero));
}
#[test]
fn scrap_trawler_rechecks_the_target_but_keeps_the_event_value_after_other_moves() {
    let mut e = setup();
    let stone = sacrifice(&mut e, "Mind Stone");
    let ring = in_graveyard(&e, P0, card("Sol Ring")).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![ring],
        },
    )
    .unwrap();
    // The event card leaves the graveyard and its former LKI is cleared.
    e.state
        .move_object(
            stone,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, card("Sol Ring")).is_some());

    let mut gone = setup();
    sacrifice(&mut gone, "Mind Stone");
    let ring = in_graveyard(&gone, P0, card("Sol Ring")).unwrap();
    gone.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![ring],
        },
    )
    .unwrap();
    gone.state
        .move_object(
            ring,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut gone, stack_is_empty);
    assert!(in_hand(&gone, P0, card("Sol Ring")).is_none());
}

#[test]
fn scrap_trawler_uses_copied_mana_value_before_the_copy_is_erased() {
    let mut e = Duel::new(9103, card("Island"))
        .battlefield(
            0,
            &[
                card("Scrap Trawler"),
                card("Krark-Clan Ironworks"),
                card("Sun Titan"),
                card("Island"),
                card("Island"),
                card("Island"),
                card("Island"),
            ],
        )
        .hand(0, &[card("Machine God's Effigy")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, card("Machine God's Effigy"));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let titan = on_battlefield(&e, P0, card("Sun Titan")).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![titan],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let effigy = on_battlefield(&e, P0, card("Machine God's Effigy")).unwrap();
    assert_eq!(
        e.state
            .object(effigy)
            .unwrap()
            .characteristics()
            .mana_cost
            .cmc(),
        6
    );
    sacrifice(&mut e, "Machine God's Effigy");
    assert_eq!(
        e.state
            .object(effigy)
            .unwrap()
            .characteristics()
            .mana_cost
            .cmc(),
        4
    );
    assert_eq!(
        targets(&e),
        vec![effigy],
        "the printed four-mana card is below its former copied six"
    );
    let before = e.snapshot_hash();
    e.trigger_queue.front_mut().unwrap().event_mana_value = Some(5);
    assert_ne!(
        before,
        e.snapshot_hash(),
        "the captured bound participates in replay state"
    );
    e.trigger_queue.front_mut().unwrap().event_mana_value = Some(6);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![effigy],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, card("Machine God's Effigy")).is_some());
}

#[test]
fn scrap_trawler_simultaneous_deaths_get_separate_mana_bounds() {
    let mut e = Duel::new(9104, card("Swamp"))
        .battlefield(
            0,
            &[
                card("Scrap Trawler"),
                card("Ornithopter"),
                card("Swamp"),
                card("Swamp"),
                card("Swamp"),
            ],
        )
        .hand(0, &[card("Toxic Deluge")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, card("Toxic Deluge"));
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let thopter = in_graveyard(&e, P0, card("Ornithopter")).unwrap();
    assert_eq!(targets(&e), vec![thopter]);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![thopter],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, card("Ornithopter")).is_some());
    assert!(in_graveyard(&e, P0, card("Scrap Trawler")).is_some());
}
