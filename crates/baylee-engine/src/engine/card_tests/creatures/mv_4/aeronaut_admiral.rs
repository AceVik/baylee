//! `cards/creatures/mv_4/aeronaut_admiral.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn aeronaut_admiral() -> CardIndex {
    card_index("dcde1a62-33a5-4454-a389-9bf7a3521a4c")
}

fn crew_conqueror_galleon_with_bears(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    activate(e, seat, index::CONQUEROR_S_GALLEON, 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("crew asks for creatures to tap: {:?}", e.pending());
    };
    let bears: Vec<_> = options
        .into_iter()
        .filter(|object| {
            e.state().object(*object).is_some_and(|o| {
                o.card
                    .is_some_and(|card| card.index == index::GRIZZLY_BEARS)
            })
        })
        .collect();
    assert_eq!(bears.len(), 2, "the two Bears are offered to pay crew 4");
    e.apply(seat, PlayerAction::ChooseObjects { objects: bears })
        .expect("the offered Bears pay for crew");
    pass_until(e, stack_is_empty);
}

// Checklist: Admiral has flying; only its controller's Vehicle gains flying;
// non-Vehicle and opponent's Vehicle do not; the granted ability changes legal blocks;
// removing Admiral ends the grant while the crewed Vehicle remains a creature.
#[test]
fn aeronaut_admiral_grants_flying_to_its_controllers_vehicles() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                aeronaut_admiral(),
                index::CONQUEROR_S_GALLEON,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
                quiet_artifact(),
                island(),
            ],
        )
        .battlefield(
            1,
            &[
                index::CONQUEROR_S_GALLEON,
                llanowar_elves(),
                snapping_drake(),
            ],
        )
        .hand(0, &[unsummon()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let admiral = on_battlefield(&e, p0, aeronaut_admiral()).expect("Admiral");
    let vehicle = on_battlefield(&e, p0, index::CONQUEROR_S_GALLEON).expect("our Vehicle");
    let ordinary = on_battlefield(&e, p0, quiet_artifact()).expect("ordinary artifact");
    let their_vehicle =
        on_battlefield(&e, p1, index::CONQUEROR_S_GALLEON).expect("opponent's Vehicle");
    assert!(keywords(&e, admiral).contains(KeywordSet::FLYING));
    assert!(keywords(&e, vehicle).contains(KeywordSet::FLYING));
    assert!(!keywords(&e, ordinary).contains(KeywordSet::FLYING));
    assert!(!keywords(&e, their_vehicle).contains(KeywordSet::FLYING));

    crew_conqueror_galleon_with_bears(&mut e, p0);

    let ground = on_battlefield(&e, p1, llanowar_elves()).expect("opponent's ground creature");
    let flier = on_battlefield(&e, p1, snapping_drake()).expect("opponent's flyer");
    let blocks = attack_and_collect_blocks(&mut e, vehicle, p1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == ground && b.attackers.contains(&vehicle))
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == flier && b.attackers.contains(&vehicle))
    );
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("the opponent declines to block");
    cast_from_hand(&mut e, p0, unsummon());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![admiral],
        },
    )
    .expect("Unsummon returns the Admiral that granted flying");
    pass_until(&mut e, stack_is_empty);
    assert!(!keywords(&e, vehicle).contains(KeywordSet::FLYING));
    let vehicle_types = e
        .state()
        .object(vehicle)
        .expect("vehicle remains")
        .characteristics()
        .types;
    assert!(vehicle_types.contains(TypeSet::ARTIFACT));
    assert!(
        vehicle_types.contains(TypeSet::CREATURE),
        "crew lasts until end of turn"
    );
    assert_eq!(
        e.state()
            .object(vehicle)
            .and_then(|o| o.card)
            .map(|c| c.index),
        Some(index::CONQUEROR_S_GALLEON)
    );
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
}
