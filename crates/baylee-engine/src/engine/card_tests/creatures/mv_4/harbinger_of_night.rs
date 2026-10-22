//! `cards/creatures/mv_4/harbinger_of_night.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HARBINGER_OF_NIGHT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Harbinger of Night (33124133-ed2c-4b86-b7fb-90470b86f5b1) — {2}{B}{B} 2/3:
/// "At the beginning of your upkeep, put a -1/-1 counter on each creature."
#[test]
fn harbinger_of_night_puts_minus_one_minus_one_counter_on_each_creature_at_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[HARBINGER_OF_NIGHT])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let harbinger = on_battlefield(&engine, P0, HARBINGER_OF_NIGHT).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    assert_eq!(
        engine
            .state()
            .object(harbinger)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        1
    );
}
