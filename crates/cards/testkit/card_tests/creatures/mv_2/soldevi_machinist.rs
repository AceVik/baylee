//! `cards/creatures/mv_2/soldevi_machinist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn soldevi_machinist() -> CardIndex {
    card_index("06600efe-1718-4cc1-b1c0-59c82e2efd73")
}

fn snow_fortress() -> CardIndex {
    card_index("7709ed84-ed71-4dd2-b002-b257418dd79e")
}

fn morphling() -> CardIndex {
    card_index("19f233b2-8be6-4172-abf9-b132fc3393d9")
}

/// Soldevi Machinist (06600efe-1718-4cc1-b1c0-59c82e2efd73) — {1}{U} 1/1 Creature — Human Wizard Artificer:
/// "{T}: Add {C}{C}. Spend this mana only to activate abilities of artifacts."
#[test]
fn soldevi_machinist_mana_activates_artifact_abilities_but_cannot_cast_spells_or_activate_nonartifacts()
 {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[soldevi_machinist(), snow_fortress(), morphling()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let machinist = on_battlefield(&engine, P0, soldevi_machinist()).unwrap();
    let fortress = on_battlefield(&engine, P0, snow_fortress()).unwrap();
    let shape = on_battlefield(&engine, P0, morphling()).unwrap();
    let hand_artifact = in_hand(&engine, P0, sol_ring()).unwrap();

    // Tap Soldevi Machinist for {C}{C} restricted to artifact abilities.
    tap_mana_where(&mut engine, P0, |id| id == machinist);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0,
        "both {{C}} in the pool carry the artifact-abilities restriction"
    );

    // 1. Non-artifact creature's {1} ability (Morphling ability 3) is not offered,
    // and {1} artifact spell in hand (Sol Ring) cannot be cast.
    let offer = priority_offer(&engine);
    assert!(
        !offer.castable.contains(&hand_artifact),
        "restricted mana cannot cast an artifact spell"
    );
    assert!(
        !offer.abilities.contains(&(shape, 3)),
        "restricted mana cannot pay a non-artifact creature's {{1}} ability"
    );
    assert!(
        offer.abilities.contains(&(fortress, 0)),
        "artifact's {{1}} ability is payable directly from Soldevi Machinist's floating mana"
    );

    // 2. Spend both {C} on Snow Fortress's {1}: +1/+0 ability twice (0/4 -> 2/4).
    activate(&mut engine, P0, snow_fortress(), 0);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, P0, snow_fortress(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fortress), (2, 4));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
