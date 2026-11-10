//! `cards/enchantments/mv_3/goblin_warrens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn warrens_card() -> CardIndex {
    card_index("c1161505-1623-4475-a88e-2e48072cc2f1")
}
fn raiders_card() -> CardIndex {
    // Mons's Goblin Raiders, a 1/1 Goblin.
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn game(creatures: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut mine = vec![warrens_card(), mountain(), mountain(), mountain()];
    mine.extend_from_slice(creatures);
    let mut e = Duel::new(SEED, swamp()).battlefield(0, &mine).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn tokens(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = e.state().object(id).expect("on the battlefield");
            !o.is_card() && o.controller == P0
        })
        .collect()
}

/// Taps for the {2}{R}, presses the Warrens, and answers the two sacrifice
/// questions with `victims`, in order. Each question must offer its victim.
#[track_caller]
fn press(e: &mut Engine<RegistryLookup>, victims: [ObjectId; 2]) {
    tap_all_mana(e, P0);
    activate(e, P0, warrens_card(), 0);
    for victim in victims {
        let Pending::ChooseCards {
            player, options, ..
        } = e.pending().clone()
        else {
            panic!("a sacrifice is a choice, got {:?}", e.pending())
        };
        assert_eq!(player, P0);
        assert!(options.contains(&victim), "the Goblin is on the menu");
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Goblin is a legal sacrifice");
    }
    pass_until(e, stack_is_empty);
}

/// Goblin Warrens (c1161505-1623-4475-a88e-2e48072cc2f1) — "{2}{R}, Sacrifice
/// two Goblins: Create three 1/1 red Goblin creature tokens." Two Goblins go,
/// three come, and a non-Goblin is never on the menu.
#[test]
fn goblin_warrens_trades_two_goblins_for_three_goblin_tokens() {
    let mut e = game(&[raiders_card(), raiders_card(), raiders_card(), bears_card()]);
    let goblins = all_on_battlefield(&e, P0, raiders_card());
    assert_eq!(goblins.len(), 3);
    let bears = on_battlefield(&e, P0, bears_card()).expect("the Bears");
    assert!(tokens(&e).is_empty());

    tap_all_mana(&mut e, P0);
    activate(&mut e, P0, warrens_card(), 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("a sacrifice is a choice, got {:?}", e.pending())
    };
    assert!(!options.contains(&bears), "the Bears are no Goblin");
    for g in &goblins {
        assert!(options.contains(g), "every Goblin is offered");
    }
    for victim in &goblins[..2] {
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![*victim],
            },
        )
        .expect("a Goblin is a legal sacrifice");
    }
    pass_until(&mut e, stack_is_empty);

    for victim in &goblins[..2] {
        assert_eq!(
            e.state().object(*victim).map(|o| o.zone),
            Some(Zone::Graveyard),
            "sacrificed"
        );
    }
    assert_eq!(
        e.state().object(goblins[2]).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the third Goblin was not needed"
    );
    let made = tokens(&e);
    assert_eq!(made.len(), 3, "three Goblin tokens");
    for t in made {
        assert_eq!(pt(&e, t), (1, 1), "each is a 1/1");
    }
}

/// Goblin Warrens — the tokens are Goblins themselves, so they feed the next
/// activation: three tokens pay two and make three again.
#[test]
fn goblin_warrens_tokens_can_be_sacrificed_to_it_again() {
    let mut e = game(&[raiders_card(), raiders_card()]);
    let goblins = all_on_battlefield(&e, P0, raiders_card());
    press(&mut e, [goblins[0], goblins[1]]);
    let made = tokens(&e);
    assert_eq!(made.len(), 3);

    // The Mountains are tapped: give the second activation its mana from the
    // next turn's untap.
    reach_their_main_phase(&mut e, PlayerId::new(1));
    reach_their_main_phase(&mut e, P0);
    press(&mut e, [made[0], made[1]]);
    assert_eq!(
        tokens(&e).len(),
        4,
        "three tokens, two sacrificed, three more made"
    );
}

/// Goblin Warrens — "Sacrifice two Goblins": a single Goblin cannot pay.
#[test]
fn goblin_warrens_needs_two_goblins() {
    let mut e = game(&[raiders_card()]);
    tap_all_mana(&mut e, P0);
    let warrens = on_battlefield(&e, P0, warrens_card()).expect("the Warrens");
    assert!(
        !priority_offer(&e).abilities.contains(&(warrens, 0)),
        "one Goblin is not two"
    );
}
