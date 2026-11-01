//! `cards/creatures/mv_4/illusionary_forces.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn illusionary_forces() -> CardIndex {
    card_index("e17e3bda-75cb-47a1-9f60-f50705a46a1e")
}

/// Illusionary Forces (e17e3bda-75cb-47a1-9f60-f50705a46a1e) — {3}{U} 4/4 Creature — Illusion:
/// - "Flying"
/// - "Cumulative upkeep {U}"
#[test]
fn illusionary_forces_flies_over_ground_blocker_and_scales_cumulative_upkeep() {
    let mut engine = lk::game_with(
        SEED,
        &[illusionary_forces(), island(), island()],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Blue, 1)]);
            lk::yes_no(e, true);
        },
    );
    let forces = lk::obj(&engine, P0, illusionary_forces());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let drake = lk::obj(&engine, P1, index::WIND_DRAKE);

    assert_eq!(pt(&engine, forces), (4, 4));
    assert_eq!(
        engine
            .state()
            .object(forces)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1,
        "first upkeep put 1 age counter on Illusionary Forces and paid {{U}}"
    );

    let offer = lk::attack(&mut engine, &[forces]);
    assert!(
        !lk::may_block(&offer, bears, forces),
        "ground Grizzly Bears cannot block flying Illusionary Forces"
    );
    assert!(
        lk::may_block(&offer, drake, forces),
        "flying Wind Drake can block Illusionary Forces"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(engine.state().players[1].life, 16);

    // Second upkeep: 2 age counters, pay {U}{U}.
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
            .object(forces)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert!(
        on_battlefield(&engine, P0, illusionary_forces()).is_some(),
        "paying {{U}}{{U}} for 2 age counters keeps Illusionary Forces on the battlefield"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Third upkeep: 3 age counters, decline payment -> sacrificed.
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
            .object(forces)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        3
    );
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(
        in_graveyard(&engine, P0, illusionary_forces()).is_some(),
        "declining cumulative upkeep sacrifices Illusionary Forces"
    );
}
