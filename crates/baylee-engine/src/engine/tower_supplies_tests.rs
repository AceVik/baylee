//! Custom supplies use normal legality, costs, priority and replay rules.

use super::testkit::{
    RegistryLookup, keep_mulligans, pass_until, reach_main_phase, stack_is_empty,
};
use super::*;
use crate::object::{CounterKind, ObjectKind};
use baylee_core::ids::CardIndex;
use baylee_core::preset::{SeatSupplies, TowerSupply};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn preset(healing: u8, mana: u8, seedling: bool) -> GamePreset {
    let mut p = super::opening_rules_tests::preset(vec![]);
    p.house_rules.supplies = vec![SeatSupplies {
        seat: 0,
        healing,
        mana,
        seedling,
    }];
    p
}

fn game(healing: u8, mana: u8) -> Engine<RegistryLookup> {
    let mut e = Engine::new(&preset(healing, mana, false), RegistryLookup).unwrap();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn potion(e: &Engine<RegistryLookup>, kind: TowerSupply) -> ObjectId {
    let definition = baylee_cards::tokens::tower::definition(kind);
    *e.state()
        .zones
        .list(ZoneLocation::Command(P0))
        .iter()
        .find(|id| {
            e.state()
                .object(**id)
                .is_some_and(|o| o.token.is_some_and(|t| std::ptr::eq(t, definition)))
        })
        .expect("potion in command zone")
}

fn drink(source: ObjectId) -> PlayerAction {
    PlayerAction::ActivateAbility {
        source,
        ability_index: 0,
    }
}

#[test]
fn potions_are_owned_command_emblems_and_seedling_is_an_ordinary_creature() {
    let mut e = Engine::new(&preset(1, 1, true), RegistryLookup).unwrap();
    for kind in [TowerSupply::HealingPotion, TowerSupply::ManaPotion] {
        let id = potion(&e, kind);
        let object = e.state().object(id).unwrap();
        assert_eq!(object.kind, ObjectKind::Emblem);
        assert_eq!(object.counters.get(CounterKind::Charge), 1);
        assert!(e.apply(P0, drink(id)).is_err(), "no use during mulligans");
    }
    let board = e.state().zones.list(ZoneLocation::Battlefield);
    assert_eq!(board.len(), 1);
    let plant = e.state().object(board[0]).unwrap();
    assert_eq!(plant.controller, P0);
    assert_eq!(plant.characteristics().power, Some(1));
    assert_eq!(plant.characteristics().toughness, Some(1));
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let id = potion(&e, TowerSupply::HealingPotion);
    assert!(!e.compute_legal(P1).abilities.contains(&(id, 0)));
    let before = e.fingerprint();
    assert!(e.apply(P1, drink(id)).is_err());
    assert_eq!(before, e.fingerprint());
}

#[test]
fn each_healing_charge_pays_once_and_uses_the_stack() {
    let mut e = game(2, 0);
    let id = potion(&e, TowerSupply::HealingPotion);
    for remaining in [1, 0] {
        assert!(e.compute_legal(P0).abilities.contains(&(id, 0)));
        let life = e.state().players[0].life;
        e.apply(P0, drink(id)).unwrap();
        assert_eq!(
            e.state()
                .object(id)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            remaining
        );
        assert_eq!(
            e.state().players[0].life,
            life,
            "healing waits on the stack"
        );
        assert!(!stack_is_empty(&e));
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, life + 5);
    }
    assert!(!e.compute_legal(P0).abilities.contains(&(id, 0)));
    let before = e.fingerprint();
    assert!(e.apply(P0, drink(id)).is_err());
    assert_eq!(before, e.fingerprint());
}

#[test]
fn mana_potion_asks_one_color_refuses_colorless_and_does_not_use_the_stack() {
    let mut e = game(0, 1);
    let id = potion(&e, TowerSupply::ManaPotion);
    assert!(e.compute_legal(P0).abilities.contains(&(id, 0)));
    e.apply(P0, drink(id)).unwrap();
    assert!(matches!(e.pending(), Pending::ChooseColor { options, .. } if options.len() == 5));
    assert!(stack_is_empty(&e));
    let before = e.fingerprint();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Colorless)
        )
        .is_err()
    );
    assert_eq!(before, e.fingerprint());
    e.apply(
        P0,
        PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue),
    )
    .unwrap();
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Blue),
        1
    );
    assert!(!e.compute_legal(P0).abilities.contains(&(id, 0)));
}

#[test]
fn supply_consumption_and_color_choices_replay_identically() {
    let mut a = game(1, 1);
    let mut b = game(1, 1);
    let id = potion(&a, TowerSupply::ManaPotion);
    for action in [
        drink(id),
        PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Red),
    ] {
        a.apply(P0, action.clone()).unwrap();
        b.apply(P0, action).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    }
    let id = potion(&a, TowerSupply::HealingPotion);
    a.apply(P0, drink(id)).unwrap();
    b.apply(P0, drink(id)).unwrap();
    pass_until(&mut a, stack_is_empty);
    pass_until(&mut b, stack_is_empty);
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
}

#[test]
fn unsupported_registry_refuses_instead_of_losing_paid_items() {
    struct NoSupplies;
    impl CardLookup for NoSupplies {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }
    assert!(matches!(
        Engine::new(&preset(1, 0, false), NoSupplies),
        Err(EngineError::Setup(
            crate::state::SetupError::UnsupportedSupply
        ))
    ));
}
