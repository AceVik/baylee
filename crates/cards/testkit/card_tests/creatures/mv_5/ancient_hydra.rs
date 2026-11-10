//! `cards/creatures/mv_5/ancient_hydra.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Ancient Hydra ({4}{R} 5/1 Creature — Hydra):
/// "Fading 5
/// {1}, Remove a fade counter from this creature: It deals 1 damage to any target."
#[test]
fn ancient_hydra_enters_with_five_fade_counters_and_spends_them_to_deal_damage() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[card_index("8e1d3722-9bd1-4ab7-b4a5-9265fca3aba9")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(
        &mut engine,
        P0,
        card_index("8e1d3722-9bd1-4ab7-b4a5-9265fca3aba9"),
    );
    pass_until(&mut engine, stack_is_empty);

    let hydra = on_battlefield(
        &engine,
        P0,
        card_index("8e1d3722-9bd1-4ab7-b4a5-9265fca3aba9"),
    )
    .unwrap();
    assert_eq!(
        engine
            .state()
            .object(hydra)
            .unwrap()
            .counters
            .get(counters::FADE),
        5
    );

    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    engine.refresh_offer();

    activate(
        &mut engine,
        P0,
        card_index("8e1d3722-9bd1-4ab7-b4a5-9265fca3aba9"),
        1,
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(hydra)
            .unwrap()
            .counters
            .get(counters::FADE),
        4
    );
    assert_eq!(engine.state().players[1].life, 19);

    // Upkeep removes another fade counter (4 -> 3).
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        engine
            .state()
            .object(hydra)
            .unwrap()
            .counters
            .get(counters::FADE),
        3
    );
}
