//! `cards/artifacts/mv_4/ice_cauldron.rs`, played.
//!
//! Oracle:
//! - "{X}, {T}: You may exile a nonland card from your hand. You may cast that card for as long as it remains exiled. Put a charge counter on this artifact and note the type and amount of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact."
//! - "{T}, Remove a charge counter from this artifact: Add this artifact's last noted type and amount of mana. Spend this mana only to cast the last card exiled with this artifact."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ice_cauldron() -> CardIndex {
    card_index("2b7e4e70-53b1-4d4f-bdcc-56c2e15f367a")
}

#[track_caller]
fn untap(engine: &mut Engine<RegistryLookup>, cauldron: ObjectId) {
    engine
        .dev_state_mut(P0)
        .unwrap()
        .set_tapped(cauldron, false);
    engine.refresh_offer();
}

#[track_caller]
fn activate_first(
    engine: &mut Engine<RegistryLookup>,
    x: u32,
    exile: bool,
    chosen: Option<ObjectId>,
) -> Vec<ObjectId> {
    activate(engine, P0, ice_cauldron(), 0);
    let Pending::ChooseNumber { player, .. } = engine.pending().clone() else {
        panic!("X must be announced, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        panic!("the exile is optional, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::YesNo(exile)).unwrap();
    let mut options = Vec::new();
    if exile {
        let Pending::ChooseCards {
            player,
            options: offered,
            min,
            max,
            ..
        } = engine.pending().clone()
        else {
            panic!(
                "choose a nonland from your hand, got {:?}",
                engine.pending()
            );
        };
        assert_eq!((min, max), (1, 1));
        options = offered;
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![chosen.expect("a card from the offered hand")],
                },
            )
            .unwrap();
    }
    lk::settle(engine);
    options
}

#[track_caller]
fn add_notted_mana(engine: &mut Engine<RegistryLookup>, cauldron: ObjectId) {
    untap(engine, cauldron);
    activate(engine, P0, ice_cauldron(), 1);
    lk::settle(engine);
}

#[track_caller]
fn charge(engine: &Engine<RegistryLookup>, cauldron: ObjectId) -> u16 {
    engine
        .state()
        .object(cauldron)
        .unwrap()
        .counters
        .get(CounterKind::Charge)
}

#[track_caller]
fn object_castable(engine: &Engine<RegistryLookup>, card: ObjectId) -> bool {
    matches!(engine.pending(), Pending::Priority { player, legal } if *player == P0 && legal.castable.contains(&card))
}

#[track_caller]
fn object_permitted(engine: &Engine<RegistryLookup>, card: ObjectId) -> bool {
    matches!(engine.pending(), Pending::Priority { player, legal } if *player == P0
        && (legal.castable.contains(&card) || legal.payable.contains(&card)))
}

#[test]
fn ice_cauldron_exiles_only_a_chosen_nonland_and_allows_declining() {
    let mut engine = lk::game(
        SEED,
        &[ice_cauldron(), index::FOREST],
        &[],
        &[index::FOREST, index::GRIZZLY_BEARS, index::GRAY_OGRE],
        &[],
    );
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let forest = in_hand(&engine, P0, index::FOREST).unwrap();
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let ogre = in_hand(&engine, P0, index::GRAY_OGRE).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);

    let options = activate_first(&mut engine, 2, true, Some(bears));
    assert_eq!(options, vec![bears, ogre]);
    assert!(!options.contains(&forest), "a nonland card, not any card");
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(ogre).unwrap().zone, Zone::Hand);
    assert_eq!(charge(&engine, cauldron), 1);
    assert!(
        baylee_engine::casting::play_permission(engine.state(), P0, bears).is_some(),
        "the permission belongs to the exact exiled incarnation"
    );
    assert!(
        object_permitted(&engine, bears),
        "the permission exists even before its mana cost is available: probe={:?}, pending={:?}",
        baylee_engine::casting::can_cast(engine.state(), &RegistryLookup, P0, bears),
        engine.pending()
    );

    untap(&mut engine, cauldron);
    assert!(!lk::offered(&engine, ice_cauldron(), P0, 0));
    add_notted_mana(&mut engine, cauldron);
    assert_eq!(charge(&engine, cauldron), 0);
    assert_eq!(lk::pool_total(&engine, P0), 2);
    untap(&mut engine, cauldron);
    activate_first(&mut engine, 0, false, None);
    assert_eq!(
        charge(&engine, cauldron),
        1,
        "declining still notes X and adds a counter"
    );
    assert_eq!(engine.state().object(ogre).unwrap().zone, Zone::Hand);
    assert_eq!(lk::pool_total(&engine, P0), 2);
}

#[test]
fn ice_cauldron_cast_permission_outlives_its_source_and_the_turn() {
    let mut engine = lk::game(SEED, &[ice_cauldron()], &[], &[index::LIGHTNING_BOLT], &[]);
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).unwrap();
    activate_first(&mut engine, 0, true, Some(bolt));
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    assert!(
        object_castable(&engine, bolt),
        "the permission has no turn limit"
    );
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            cauldron,
            ZoneLocation::Graveyard(P0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    assert!(
        object_castable(&engine, bolt),
        "the exiled card, not its source, holds the permission"
    );
    engine
        .apply(P0, PlayerAction::CastSpell { card: bolt })
        .expect("cast after Ice Cauldron has left");
    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 17);
}

#[test]
fn ice_cauldron_adds_actual_types_spent_and_only_that_card_may_use_them() {
    let mut engine = lk::game(
        SEED,
        &[ice_cauldron()],
        &[],
        &[index::GRIZZLY_BEARS, index::GRAY_OGRE],
        &[],
    );
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let ogre = in_hand(&engine, P0, index::GRAY_OGRE).unwrap();
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Red, 2), (ManaColor::Colorless, 1)],
    );
    activate_first(&mut engine, 3, true, Some(bears));
    add_notted_mana(&mut engine, cauldron);
    assert_eq!(charge(&engine, cauldron), 0);
    let restricted: Vec<_> = engine.state().players[0]
        .mana_pool
        .restricted()
        .iter()
        .map(|mana| (mana.color, mana.amount, mana.restriction))
        .collect();
    assert_eq!(restricted.len(), 2);
    assert!(
        restricted
            .iter()
            .any(|(color, amount, _)| *color == ManaColor::Red && *amount == 2)
    );
    assert!(
        restricted
            .iter()
            .any(|(color, amount, _)| *color == ManaColor::Colorless && *amount == 1)
    );
    assert!(
        !object_castable(&engine, ogre),
        "the other spell is payable with three mana, but not with this restricted mana"
    );
    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    engine
        .apply(P0, PlayerAction::CastSpell { card: bears })
        .expect("the exiled card may spend both its restricted mana and ordinary mana");
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
    let remaining: Vec<_> = engine.state().players[0]
        .mana_pool
        .restricted()
        .iter()
        .map(|mana| (mana.color, mana.amount, mana.restriction))
        .collect();
    assert!(remaining.iter().all(|(color, amount, restriction)| {
        restricted
            .iter()
            .any(|(initial_color, initial_amount, initial_restriction)| {
                color == initial_color
                    && restriction == initial_restriction
                    && amount <= initial_amount
            })
    }));
    let before: u16 = restricted.iter().map(|(_, amount, _)| *amount).sum();
    let after: u16 = remaining.iter().map(|(_, amount, _)| *amount).sum();
    assert_eq!(before - after, 1, "one noted mana pays the generic cost");
    assert_eq!(lk::pool_total(&engine, P0), 2);
}

#[test]
fn ice_cauldron_second_cycle_replaces_only_the_mana_note_and_last_exile() {
    let mut engine = lk::game(
        SEED,
        &[ice_cauldron(), index::FOREST],
        &[],
        &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS, index::GRAY_OGRE],
        &[],
    );
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let ogre = in_hand(&engine, P0, index::GRAY_OGRE).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    activate_first(&mut engine, 2, true, Some(bears));
    let second_bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert_ne!(bears, second_bears);
    add_notted_mana(&mut engine, cauldron);
    engine.dev_state_mut(P0).unwrap().players[0].mana_pool = baylee_core::mana::ManaPool::new();
    untap(&mut engine, cauldron);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    activate_first(&mut engine, 1, true, Some(ogre));
    add_notted_mana(&mut engine, cauldron);
    let restricted: Vec<_> = engine.state().players[0]
        .mana_pool
        .restricted()
        .iter()
        .map(|mana| (mana.color, mana.amount))
        .collect();
    assert_eq!(restricted, vec![(ManaColor::Red, 1)]);
    assert!(
        object_permitted(&engine, bears),
        "the older exile keeps its permission"
    );
    assert!(
        baylee_engine::casting::play_permission(engine.state(), P0, second_bears).is_none(),
        "a same-named copy has no linked-exile permission"
    );
    assert!(
        !object_castable(&engine, second_bears),
        "the last exile's mana cannot cast a same-named copy"
    );
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    engine
        .apply(P0, PlayerAction::CastSpell { card: ogre })
        .expect("only the last exile may spend the new note");
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, index::GRAY_OGRE).is_some());
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    engine
        .apply(P0, PlayerAction::CastSpell { card: bears })
        .expect("the older card remains independently castable");
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn ice_cauldron_zero_note_adds_no_mana_and_can_start_a_new_cycle() {
    let mut engine = lk::game(SEED, &[ice_cauldron()], &[], &[], &[]);
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    activate_first(&mut engine, 0, false, None);
    assert_eq!(charge(&engine, cauldron), 1);
    add_notted_mana(&mut engine, cauldron);
    assert_eq!(charge(&engine, cauldron), 0);
    assert_eq!(lk::pool_total(&engine, P0), 0);
    untap(&mut engine, cauldron);
    assert!(lk::offered(&engine, ice_cauldron(), P0, 0));
}

#[track_caller]
fn move_to(engine: &mut Engine<RegistryLookup>, card: ObjectId, to: ZoneLocation) {
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            card,
            to,
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
}

#[test]
fn ice_cauldron_permission_and_payment_end_when_the_exile_moves() {
    let mut engine = lk::game(SEED, &[ice_cauldron()], &[], &[index::LIGHTNING_BOLT], &[]);
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    activate_first(&mut engine, 1, true, Some(bolt));
    add_notted_mana(&mut engine, cauldron);
    let source = engine.state().object(cauldron).unwrap();
    let linked = source
        .activation_mana_note(source.rules_acquisition())
        .and_then(|(_, exile)| exile)
        .expect("the exiled incarnation is linked to the mana");
    assert_eq!(linked.object, bolt);

    move_to(&mut engine, bolt, ZoneLocation::Graveyard(P0));
    assert!(baylee_engine::casting::play_permission(engine.state(), P0, bolt).is_none());
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: bolt })
            .is_err()
    );
    assert_eq!(
        lk::pool_total(&engine, P0),
        1,
        "a refused cast spends nothing"
    );

    move_to(&mut engine, bolt, ZoneLocation::Exile(P0));
    assert_ne!(engine.state().object(bolt).unwrap().version, linked.version);
    assert!(
        baylee_engine::casting::play_permission(engine.state(), P0, bolt).is_none(),
        "returning the physical card does not restore its old permission (CR 400.7)"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: bolt })
            .is_err()
    );
    assert_eq!(
        lk::pool_total(&engine, P0),
        1,
        "the old mana cannot pay the new incarnation"
    );
    let source = engine.state().object(cauldron).unwrap();
    assert_eq!(
        source
            .activation_mana_note(source.rules_acquisition())
            .and_then(|(_, exile)| exile),
        Some(linked),
        "the note still names the old incarnation, which no longer exists"
    );
}

#[test]
fn ice_cauldron_a_new_source_incarnation_forgets_only_its_mana_note() {
    let mut engine = lk::game(SEED, &[ice_cauldron()], &[], &[index::LIGHTNING_BOLT], &[]);
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).unwrap();
    activate_first(&mut engine, 0, true, Some(bolt));
    let old_version = engine.state().object(cauldron).unwrap().version;
    assert!(baylee_engine::resolve::noted_activation_mana(engine.state(), cauldron).is_some());

    move_to(&mut engine, cauldron, ZoneLocation::Graveyard(P0));
    assert!(baylee_engine::resolve::noted_activation_mana(engine.state(), cauldron).is_none());
    assert!(
        baylee_engine::casting::play_permission(engine.state(), P0, bolt).is_some(),
        "the independent cast permission remains with the exiled card"
    );

    move_to(&mut engine, cauldron, ZoneLocation::Battlefield);
    assert_ne!(
        engine.state().object(cauldron).unwrap().version,
        old_version
    );
    assert!(
        baylee_engine::resolve::noted_activation_mana(engine.state(), cauldron).is_none(),
        "a new permanent has no last noted mana (CR 400.7)"
    );
    assert!(baylee_engine::casting::play_permission(engine.state(), P0, bolt).is_some());
}

#[test]
fn ice_cauldron_declining_a_later_exile_preserves_the_payment_target() {
    let mut engine = lk::game(SEED, &[ice_cauldron()], &[], &[index::GRIZZLY_BEARS], &[]);
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    activate_first(&mut engine, 1, true, Some(bears));
    add_notted_mana(&mut engine, cauldron);
    engine.dev_state_mut(P0).unwrap().players[0].mana_pool = baylee_core::mana::ManaPool::new();
    let source = engine.state().object(cauldron).unwrap();
    let previous_exile = source
        .activation_mana_note(source.rules_acquisition())
        .and_then(|(_, exile)| exile);

    untap(&mut engine, cauldron);
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 1)]);
    activate_first(&mut engine, 1, false, None);
    let source = engine.state().object(cauldron).unwrap();
    let (types, last_exile) = source
        .activation_mana_note(source.rules_acquisition())
        .expect("the new activation replaces its mana note");
    assert_eq!(types, [0, 0, 0, 0, 0, 1]);
    assert_eq!(
        last_exile, previous_exile,
        "declining changes no exile link"
    );

    add_notted_mana(&mut engine, cauldron);
    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    engine
        .apply(P0, PlayerAction::CastSpell { card: bears })
        .expect("the old exile may spend the newly noted mana");
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn ice_cauldron_refuses_other_spells_and_activated_ability_costs() {
    let mut engine = lk::game(
        SEED,
        &[ice_cauldron(), index::ELKIN_BOTTLE],
        &[],
        &[index::ELKIN_BOTTLE, index::ELKIN_BOTTLE],
        &[],
    );
    let cauldron = lk::obj(&engine, P0, ice_cauldron());
    let bottle = lk::obj(&engine, P0, index::ELKIN_BOTTLE);
    let linked_bottle = in_hand(&engine, P0, index::ELKIN_BOTTLE).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    activate_first(&mut engine, 3, true, Some(linked_bottle));
    let other_bottle = in_hand(&engine, P0, index::ELKIN_BOTTLE).unwrap();
    assert_ne!(linked_bottle, other_bottle);
    add_notted_mana(&mut engine, cauldron);
    assert_eq!(lk::pool_total(&engine, P0), 3);
    assert!(
        object_castable(&engine, linked_bottle),
        "the linked spell can pay its wholly generic cost before the negative probes"
    );

    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: other_bottle })
            .is_err(),
        "the same-named spell has the same {{3}} cost, but a different object identity"
    );
    assert_eq!(
        engine.state().object(other_bottle).unwrap().zone,
        Zone::Hand
    );
    assert_eq!(lk::pool_total(&engine, P0), 3);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: bottle,
                    ability_index: 0,
                },
            )
            .is_err(),
        "spell-only mana cannot pay Elkin Bottle's {{3}} activation"
    );
    assert_eq!(lk::pool_total(&engine, P0), 3);

    engine
        .apply(
            P0,
            PlayerAction::CastSpell {
                card: linked_bottle,
            },
        )
        .expect("the linked spell can spend the same mana");
    lk::settle(&mut engine);
    assert_eq!(
        engine.state().object(linked_bottle).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(lk::pool_total(&engine, P0), 0);
}
