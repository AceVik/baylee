//! `cards/enchantments/auras/mv_2/animate_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Animate Dead's "enchant creature card in a graveyard" is its spell's
/// target (CR 303.4a): with no creature card in any graveyard it has no
/// legal target and cannot be cast at all (CR 601.2c). Its return is played
/// in `enchantments::aura_bindings`.
#[test]
fn animate_dead_without_a_graveyard_creature_cannot_be_cast() {
    let p0 = PlayerId::new(0);
    let animate_dead = card_index("c0d8fef4-65f4-4769-982d-b397d2b7e977");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[animate_dead])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, animate_dead).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("main-phase priority");
    };
    assert!(
        !legal.castable.contains(&spell),
        "no creature card to enchant"
    );
}

/// "Merchant of Secrets": "When this creature enters, draw a card."
fn merchant_of_secrets_card() -> CardIndex {
    card_index("f6aebd42-0150-4741-84c2-4c85893640e9")
}

/// The creature Animate Dead returns *enters* (CR 603.6a): its own enters
/// trigger fires on an arrival from the graveyard under its new controller.
///
/// The Merchant is buried first, so the card in hand afterwards can only be
/// the trigger of that arrival; Animate Dead itself leaves the hand.
#[test]
fn animate_dead_returns_a_creature_whose_enters_trigger_fires() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), merchant_of_secrets_card()])
        .hand(0, &[animate_dead_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let merchant = on_battlefield(&engine, p0, merchant_of_secrets_card()).expect("seated");
    bury(&mut engine, &[merchant]);
    let dead = in_graveyard(&engine, p0, merchant_of_secrets_card()).expect("buried");
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let before = hand(&engine);
    assert_eq!(before, 1, "only Animate Dead is in hand");

    cast_from_hand(&mut engine, p0, animate_dead_card());
    aim_at(&mut engine, p0, dead);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, merchant_of_secrets_card()).is_some(),
        "the Merchant is back"
    );
    assert_eq!(
        hand(&engine),
        before - 1 + 1,
        "Animate Dead left the hand and the returned Merchant drew one"
    );
}

fn animate_dead_card() -> CardIndex {
    card_index("c0d8fef4-65f4-4769-982d-b397d2b7e977")
}
