//! `cards/enchantments/mv_2/illusionary_terrain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn illusionary_terrain() -> CardIndex {
    card_index("17ae63fa-fd63-45e6-81c1-bf30851bf77c")
}

/// Illusionary Terrain (17ae63fa-fd63-45e6-81c1-bf30851bf77c) — {U}{U} Enchantment:
/// - "Cumulative upkeep {2}"
#[test]
fn illusionary_terrain_cumulative_upkeep_pays_two_per_age_counter_or_sacrifices() {
    let mut engine = lk::game_with(SEED, &[illusionary_terrain()], &[], &[], &[], |e| {
        lk::float(e, P0, &[(ManaColor::Colorless, 2)]);
        lk::yes_no(e, true);
    });
    let terrain = lk::obj(&engine, P0, illusionary_terrain());
    assert_eq!(
        engine
            .state()
            .object(terrain)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1,
        "first upkeep puts 1 age counter and pays {{2}}"
    );

    // Second upkeep: 2 age counters -> {4} required; pay {4}.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (3, P0),
        "cumulative upkeep asks in its controller's own upkeep, not in the opponent's"
    );
    assert_eq!(
        engine
            .state()
            .object(terrain)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 4)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert!(
        on_battlefield(&engine, P0, illusionary_terrain()).is_some(),
        "paying {{4}} for 2 age counters keeps Illusionary Terrain on the battlefield"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Third upkeep: 3 age counters -> decline payment -> sacrificed.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (5, P0),
        "cumulative upkeep asks in its controller's own upkeep, not in the opponent's"
    );
    assert_eq!(
        engine
            .state()
            .object(terrain)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        3
    );
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(
        in_graveyard(&engine, P0, illusionary_terrain()).is_some(),
        "declining cumulative upkeep sacrifices Illusionary Terrain"
    );
}
