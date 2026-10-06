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
