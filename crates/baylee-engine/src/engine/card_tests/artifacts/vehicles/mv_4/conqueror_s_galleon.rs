//! `cards/artifacts/vehicles/mv_4/conqueror_s_galleon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn conqueror_s_galleon_casts_as_uncrewed_vehicle_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(306, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[conqueror_s_galleon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, conqueror_s_galleon());
    pass_until(&mut engine, stack_is_empty);

    let galleon = on_battlefield(&engine, p0, conqueror_s_galleon())
        .expect("Conqueror's Galleon on battlefield");
    assert_eq!(pt(&engine, galleon), (2, 10), "printed 2/10 body");

    let t = types(&engine, galleon);
    assert!(t.contains(TypeSet::ARTIFACT), "Galleon is an artifact");
    assert!(
        !t.contains(TypeSet::CREATURE),
        "uncrewed vehicle is not a creature"
    );
    assert!(!t.contains(TypeSet::LAND), "Galleon is not a land");

    // Float mana from remaining Plains and verify that no crew ability is offered.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == galleon),
        "no creatures are available to pay crew 4"
    );

    // Advance to combat and verify that Galleon cannot attack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&galleon),
        "uncrewed vehicle cannot be declared as an attacker"
    );
}

#[test]
fn galleon_crews_and_returns_transformed_only_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                conqueror_s_galleon(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ship = on_battlefield(&e, p0, conqueror_s_galleon()).unwrap();
    let before = identity(&e, ship);
    activate(&mut e, p0, conqueror_s_galleon(), 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("crew")
    };
    assert_eq!(options.len(), 2);
    e.apply(p0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(types(&e, ship).contains(TypeSet::CREATURE));
    attack_and_collect_blocks(&mut e, ship, p1);
    assert_eq!(e.state().object(ship).unwrap().face_index, 0);
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(),Pending::Priority{player,..} if *player==p0)
    });
    assert_eq!(e.state().object(ship).unwrap().face_index, 1);
    assert_ne!(identity(&e, ship), before);
    assert!(types(&e, ship).contains(TypeSet::LAND));
    assert!(!types(&e, ship).contains(TypeSet::CREATURE));
    assert!(!is_tapped(&e, ship));
    activate(&mut e, p0, conqueror_s_galleon(), 0);
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
}

/// The back face's three payment-gated abilities: loot for {2}, draw for {4},
/// and return a card from the graveyard to hand for {6}.
#[test]
fn conqueror_s_foothold_loots_draws_and_returns_a_graveyard_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[conqueror_s_galleon()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let foothold = on_battlefield(&e, p0, conqueror_s_galleon()).unwrap();
    let def = baylee_cards::by_index(conqueror_s_galleon()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(foothold, def, 1));
    e.refresh_offer();
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let untap = |e: &mut Engine<RegistryLookup>| {
        e.dev_state_mut(p0).unwrap().set_tapped(foothold, false);
        e.refresh_offer();
    };
    let give = |e: &mut Engine<RegistryLookup>, n: u32| {
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, n);
        e.refresh_offer();
    };

    // {4}, {T}: draw a card.
    let before = hand(&e);
    give(&mut e, 4);
    activate(&mut e, p0, conqueror_s_galleon(), 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(hand(&e), before + 1, "{{4}},{{T}} draws exactly one card");
    assert!(is_tapped(&e, foothold));

    // {2}, {T}: draw a card, then discard a card.
    untap(&mut e);
    let before = hand(&e);
    give(&mut e, 2);
    activate(&mut e, p0, conqueror_s_galleon(), 1);
    pass_until(&mut e, |e| {
        !matches!(e.pending(), Pending::Priority { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = e.pending().clone() {
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(hand(&e), before, "draw one, discard one");
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "the discarded card is in the graveyard"
    );

    // {6}, {T}: return target card from your graveyard to your hand.
    untap(&mut e);
    let card = e.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let before = hand(&e);
    give(&mut e, 6);
    activate(&mut e, p0, conqueror_s_galleon(), 3);
    aim_at(&mut e, p0, card);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(hand(&e), before + 1);
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Hand);
}
