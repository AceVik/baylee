//! `cards/enchantments/auras/mv_3/hobble.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Hobble — {2}{W} Enchantment — Aura
/// "Enchant creature
/// When this Aura enters, draw a card.
/// Enchanted creature can't attack.
/// Enchanted creature can't block if it's black."
#[test]
fn hobble_draws_on_etb_and_stops_black_enchanted_creature_from_attacking_and_blocking() {
    let hobble = card_index("c2398875-0912-4210-90bd-b2d568a580cf");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[hobble])
        .battlefield(1, &[index::SCATHE_ZOMBIES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let zombies_id = on_battlefield(&engine, p1, index::SCATHE_ZOMBIES).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, hobble);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![zombies_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "Hobble was cast (-1) and drew a card on ETB (+1)"
    );
    let chars = engine
        .state()
        .object(zombies_id)
        .unwrap()
        .characteristics()
        .clone();
    assert!(chars.keywords.contains(KeywordSet::CANT_ATTACK));
    assert!(
        chars.keywords.contains(KeywordSet::CANT_BLOCK),
        "black enchanted creature also gains CANT_BLOCK"
    );
}

#[test]
fn hobble_allows_nonblack_enchanted_creature_to_block() {
    let hobble = card_index("c2398875-0912-4210-90bd-b2d568a580cf");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[hobble])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears_id = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, hobble);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(bears_id)
        .unwrap()
        .characteristics()
        .clone();
    assert!(chars.keywords.contains(KeywordSet::CANT_ATTACK));
    assert!(
        !chars.keywords.contains(KeywordSet::CANT_BLOCK),
        "nonblack enchanted creature can still block"
    );
}
