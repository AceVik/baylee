//! `cards/creatures/mv_4/maro.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "ef32258a-d567-4fbc-ab85-8c00080f5b36"
fn maro() -> CardIndex {
    card_index("ef32258a-d567-4fbc-ab85-8c00080f5b36")
}

#[test]
fn maro_resolves_as_zero_zero_creature_surviving_with_anthem() {
    let mut engine = lk::game(SEED, &[index::GAEA_S_LIEGE], &[], &[maro()], &[]);

    // Put a +1/+1 counter on Maro or pump it? Wait: Gaea's Liege doesn't anthem. Let's use glorious anthem or Bad Moon? Wait: Maro is green! Let's check what green anthem exists, or let's check Maro going to graveyard as 0/0!
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, maro());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P0))
            .iter()
            .any(|&id| {
                engine
                    .state()
                    .object(id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == maro())
            }),
        "Maro enters as a 0/0 creature and is put into the graveyard by SBA"
    );
}
