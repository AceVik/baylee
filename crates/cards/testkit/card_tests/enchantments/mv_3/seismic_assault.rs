//! `cards/enchantments/mv_3/seismic_assault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seismic Assault is an enchantment under `Coverage::Implemented` costing {R}{R}{R} that deals 2 damage to any target for discarding a land.
/// Following `CR 601.2c` and `CR 601.2h`, the target is announced first, followed by the discard cost.
/// The cost menu is prompted with `ChoicePrompt::CostDiscard` and restricted to land cards in hand, excluding non-land cards.
/// Resolving the ability deals 2 damage to the chosen target and puts the discarded land card into the graveyard.
#[test]
fn seismic_assault_discards_a_land_to_deal_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[seismic_assault()])
        .hand(0, &[mountain(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land_card = in_hand(&engine, p0, mountain()).expect("Mountain is in hand");
    let non_land_card = in_hand(&engine, p0, llanowar_elves()).expect("Elf is in hand");

    activate(&mut engine, p0, seismic_assault(), 0);

    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "one target required");
    assert!(
        player_options.contains(&p1),
        "defending player is an offered target: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeting player 1 is legal");

    let Pending::ChooseCards {
        player: cost_player,
        options,
        min: cost_min,
        max: cost_max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected discard cost prompt, got {:?}", engine.pending())
    };
    assert_eq!(cost_player, p0, "controller pays discard cost");
    assert_eq!((cost_min, cost_max), (1, 1), "exactly one card discarded");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "prompt is flagged as a discard cost"
    );
    assert!(
        options.contains(&land_card),
        "land in hand is on the discard menu: {options:?}"
    );
    assert!(
        !options.contains(&non_land_card),
        "non-land card is excluded from discard menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land_card],
            },
        )
        .expect("discarding land is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two damage dealt to player 1"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "player 0 life unchanged"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "discarded land is in graveyard"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "non-land card remains in hand"
    );
    assert!(
        on_battlefield(&engine, p0, seismic_assault()).is_some(),
        "Seismic Assault remains on battlefield"
    );
}
