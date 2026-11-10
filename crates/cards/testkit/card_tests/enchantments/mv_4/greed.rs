//! `cards/enchantments/mv_4/greed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Greed` is an enchantment under `Coverage::Implemented` with an activated ability costing `{B}` and 2 life to draw a card.
/// With an empty mana pool, its ability is unpayable and is withheld from legal actions.
/// Tapping a Swamp provides `{B}`, allowing the ability to activate and deduct 2 life and `{B}` as its cost.
/// Upon resolution, a card is drawn from the library into the controller's hand.
#[test]
fn greed_pays_black_mana_and_two_life_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), greed()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, greed()).expect("Greed is on battlefield");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(enchantment, 0)),
        "Greed cannot be activated without {{B}} in the mana pool"
    );

    let initial_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let initial_lib = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps produce two black mana"
    );

    activate(&mut engine, p0, greed(), 0);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "paying 2 life is part of the activation cost"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one {{B}} was spent from the pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand + 1,
        "drew one card into hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        initial_lib - 1,
        "one card was drawn from library"
    );
    assert!(
        !is_tapped(&engine, enchantment),
        "Greed does not tap to activate"
    );
}
