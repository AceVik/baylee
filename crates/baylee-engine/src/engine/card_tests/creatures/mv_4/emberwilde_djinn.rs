//! `cards/creatures/mv_4/emberwilde_djinn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::EMBERWILDE_DJINN;

const P0: PlayerId = PlayerId::new(0);

/// Emberwilde Djinn (3e1a7146-75d5-499f-8718-303bc6ea139b) — {2}{R}{R} 5/4:
/// "Flying. At the beginning of each player's upkeep, that player may pay
/// {R}{R} or 2 life. If a player does, they gain control of this creature."
#[test]
fn emberwilde_djinn_casts_and_enters_with_flying() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[EMBERWILDE_DJINN])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, EMBERWILDE_DJINN);
    pass_until(&mut engine, stack_is_empty);

    let djinn = on_battlefield(&engine, P0, EMBERWILDE_DJINN).unwrap();
    assert_eq!(pt(&engine, djinn), (5, 4));
    assert!(
        engine
            .state()
            .object(djinn)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
}
