//! `cards/enchantments/mv_2/soul_echo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

fn soul_echo() -> CardIndex {
    card_index("8bc5546e-554f-430d-80b1-99578b5ea188")
}

/// Soul Echo (`{X}{W}{W}`, `Coverage::Partial`): enters with X echo counters,
/// prevents you from losing the game for having 0 or less life, and sacrifices
/// itself at your upkeep if there are no echo counters on it.
#[test]
fn soul_echo_enters_with_x_counters_prevents_zero_life_loss_and_sacrifices_at_zero_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Cast with X = 2: enters with 2 echo counters and stays on the battlefield at upkeep.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 4])
        .hand(0, &[soul_echo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, soul_echo());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let echo = on_battlefield(&engine, p0, soul_echo()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(echo)
            .unwrap()
            .counters
            .iter()
            .find(|(k, _)| *k == counters::ECHO)
            .map_or(0, |(_, n)| n),
        2
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, soul_echo()).is_some(),
        "Soul Echo remains on the battlefield when it has echo counters"
    );

    // Cast with X = 0: sacrifices itself at the beginning of your upkeep.
    let mut engine0 = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 2])
        .hand(0, &[soul_echo()])
        .start();
    keep_mulligans(&mut engine0);
    reach_main_phase(&mut engine0, p0);

    tap_all_mana(&mut engine0, p0);
    cast_with_floating(&mut engine0, p0, soul_echo());
    engine0.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    pass_until(&mut engine0, stack_is_empty);
    assert!(on_battlefield(&engine0, p0, soul_echo()).is_some());

    reach_their_main_phase(&mut engine0, p1);
    reach_their_main_phase(&mut engine0, p0);
    assert!(
        on_battlefield(&engine0, p0, soul_echo()).is_none(),
        "Soul Echo sacrifices itself at upkeep when it has no echo counters"
    );
}
