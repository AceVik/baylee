//! `cards/enchantments/mv_4/illusions_of_grandeur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn illusions_of_grandeur() -> CardIndex {
    card_index("d7c81b3e-8082-4aaf-8979-2717d118234c")
}

/// Illusions of Grandeur (d7c81b3e-8082-4aaf-8979-2717d118234c) — {3}{U} Enchantment:
/// - "Cumulative upkeep {2}"
/// - "When this enchantment enters, you gain 20 life."
/// - "When this enchantment leaves the battlefield, you lose 20 life."
#[test]
fn illusions_of_grandeur_gains_twenty_life_on_entry_scales_upkeep_and_loses_twenty_on_leave() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[illusions_of_grandeur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, illusions_of_grandeur());
    pass_until(&mut engine, stack_is_empty);

    let illusions =
        on_battlefield(&engine, P0, illusions_of_grandeur()).expect("Illusions entered");
    assert_eq!(
        engine.state().players[0].life,
        40,
        "ETB trigger gains 20 life"
    );

    // First upkeep: 1 age counter, pay {2}.
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
            .object(illusions)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, illusions_of_grandeur()).is_some());
    assert_eq!(engine.state().players[0].life, 40);

    // Second upkeep: 2 age counters, pay {4}.
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
            .object(illusions)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 4)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, illusions_of_grandeur()).is_some());

    // Third upkeep: 3 age counters, decline payment -> sacrificed -> LTB trigger loses 20 life.
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
        (7, P0),
        "cumulative upkeep asks in its controller's own upkeep, not in the opponent's"
    );
    assert_eq!(
        engine
            .state()
            .object(illusions)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        3
    );
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, illusions_of_grandeur()).is_some());
    assert_eq!(
        engine.state().players[0].life,
        20,
        "leaving the battlefield triggers 20 life loss"
    );
}
