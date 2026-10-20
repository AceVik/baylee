//! `cards/enchantments/mv_4/drought.rs`, played.
//!
//! Oracle:
//! - "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}."
//! - "Spells cost an additional 'Sacrifice a Swamp' to cast for each black mana symbol in their mana costs."
//! - "Activated abilities cost an additional 'Sacrifice a Swamp' to activate for each black mana symbol in their activation costs."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "ecba74bd-850e-4cbe-9b1c-b0d4d6f27d1c"
fn drought() -> CardIndex {
    card_index("ecba74bd-850e-4cbe-9b1c-b0d4d6f27d1c")
}

#[test]
fn drought_requires_paying_two_white_at_your_upkeep_or_is_sacrificed() {
    let mut engine = lk::game(SEED, &[], &[], &[drought()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, drought());
    pass_until(&mut engine, stack_is_empty);

    // Advance to P0's next upkeep and pay {W}{W}.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay {W}{W} to keep Drought");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        on_battlefield(&engine, P0, drought()).is_some(),
        "Drought remains on the battlefield when {{W}}{{W}} is paid"
    );

    // Advance to P0's following upkeep and decline paying {W}{W}: Drought is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline paying {W}{W}");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        on_battlefield(&engine, P0, drought()).is_none(),
        "Drought is sacrificed when {{W}}{{W}} is not paid"
    );
}
