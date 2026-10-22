//! `cards/instants/mv_1/desperate_gambit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DESPERATE_GAMBIT, HILL_GIANT};

/// `Desperate Gambit` ({R}, Instant) flips a coin and prevents the next damage
/// from a chosen source on a lost flip.
#[test]
fn desperate_gambit_resolves_coin_flip_and_prevents_damage_on_lost_flip() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[DESPERATE_GAMBIT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, DESPERATE_GAMBIT);
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    if let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    {
        engine
            .apply(
                p0,
                PlayerAction::ChooseDamageSource {
                    choice,
                    source: options[0],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }

    assert!(in_graveyard(&engine, p0, DESPERATE_GAMBIT).is_some());
}
