//! Tests for High Seas.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "9100f213-f4c1-4b15-94f2-e280579894c0"
fn high_seas() -> CardIndex {
    card_index("9100f213-f4c1-4b15-94f2-e280579894c0")
}

/// Red creature spells and green creature spells cost {1} more to cast.
#[test]
fn high_seas_taxes_red_and_green_creature_spells_by_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[high_seas(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    // Spend 3 Islands on High Seas ({2}{U}), leaving 3 Mountains untapped.
    let islands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == island()))
        })
        .collect();
    for id in islands {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: id })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, high_seas());
    pass_until(&mut engine, stack_is_empty);

    // Gray Ogre ({2}{R}) now costs {3}{R}, so 3 Mountains cannot pay for it.
    tap_all_mana(&mut engine, p0);
    let ogre_in_hand = in_hand(&engine, p0, gray_ogre()).expect("Gray Ogre in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: ogre_in_hand })
            .is_err()
    );
}
