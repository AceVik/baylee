//! `cards/instants/mv_14/blinkmoth_infusion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "8788f555-8c18-4842-9716-385afae2b9a4"
fn blinkmoth_infusion() -> CardIndex {
    card_index("8788f555-8c18-4842-9716-385afae2b9a4")
}

/// Blinkmoth Infusion ({12}{U}{U}, Affinity for artifacts): reduces generic
/// cost by {1} per artifact YOU control (ignoring opponent's artifacts) and
/// untaps ALL artifacts on the battlefield (both yours and opponent's), while
/// leaving tapped non-artifacts (such as lands) tapped.
#[test]
fn blinkmoth_infusion_applies_affinity_and_untaps_all_artifacts() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                sol_ring(),
                sol_ring(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[blinkmoth_infusion()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    // P1 taps their Sol Ring so we observe Blinkmoth Infusion untapping it too.
    let their_ring = lk::obj(&engine, P1, sol_ring());
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: their_ring,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(is_tapped(&engine, their_ring));

    // P0 controls 10 artifacts -> {12}{U}{U} reduces to {2}{U}{U} (4 mana).
    // Floating {1}{U}{U} (3 mana) is insufficient (P1's artifact does NOT count).
    let my_islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| my_islands.contains(&id));
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    assert!(
        !lk::castable(&engine, P0, blinkmoth_infusion()),
        "3 mana is short of Affinity-reduced cost {{2}}{{U}}{{U}}"
    );

    // Tap one of P0's Sol Rings for {C}{C} -> now 5 mana floating ({2}{U}{U} payable).
    let my_ring = all_on_battlefield(&engine, P0, sol_ring())[0];
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: my_ring,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, my_ring));

    cast_with_floating(&mut engine, P0, blinkmoth_infusion());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, my_ring),
        "your tapped artifact untapped"
    );
    assert!(
        !is_tapped(&engine, their_ring),
        "opponent's tapped artifact untapped"
    );
    assert!(
        is_tapped(&engine, my_islands[0]),
        "non-artifact tapped Island stayed tapped"
    );
}
