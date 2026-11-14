//! `cards/2002/src/ons/aura_extraction.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aura_extraction() -> CardIndex {
    card_index("84100106-1f97-4859-aadd-a35796ee000f")
}

/// One copy puts target enchantment on top of its owner's library (declining
/// artifacts and creatures); the remaining {2} cycles the second copy away to
/// draw a card.
#[test]
fn aura_extraction_tops_enchantment_and_cycles_for_two() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 4])
        .battlefield(1, &[their_enchantment(), quiet_artifact(), grizzly_bears()])
        .hand(0, &[aura_extraction(), aura_extraction()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let chant = on_battlefield(&engine, P1, their_enchantment()).unwrap();
    let ring = on_battlefield(&engine, P1, quiet_artifact()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, aura_extraction());
    let options = aim_at(&mut engine, P0, chant);
    assert!(options.contains(&chant));
    assert!(!options.contains(&ring) && !options.contains(&bears));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, their_enchantment()).is_none());
    assert!(in_graveyard(&engine, P1, their_enchantment()).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(chant)
    );

    // Remaining {2} in pool cycles the second copy from hand.
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    let lib_before = library_size(&engine, P0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let second = in_hand(&engine, P0, aura_extraction()).expect("second copy in hand");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|(src, idx)| *src == second && *idx == 0)
    );

    activate(&mut engine, P0, aura_extraction(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(library_size(&engine, P0), lib_before - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2,
        "resolved spell and cycled copy both sit in P0's graveyard"
    );
}
