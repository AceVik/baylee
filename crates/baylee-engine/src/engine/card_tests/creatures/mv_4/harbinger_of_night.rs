//! `cards/creatures/mv_4/harbinger_of_night.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn harbinger_of_night() -> CardIndex {
    card_index("9827d01d-c822-4f1b-9042-a2092c72c914")
}

/// Harbinger of Night (9827d01d-c822-4f1b-9042-a2092c72c914) — {2}{B}{B} 2/3:
/// "At the beginning of your upkeep, put a -1/-1 counter on each creature."
#[test]
fn harbinger_of_night_puts_minus_one_minus_one_counter_on_each_creature_at_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[harbinger_of_night()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let harbinger = on_battlefield(&engine, P0, harbinger_of_night()).unwrap();
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

/// "your upkeep": the opponent's upkeep puts nothing on anything (the counters
/// are still 1 each in P1's main phase), and the next upkeep of its own makes
/// a second counter, which kills the Grizzly Bears.
#[test]
fn harbinger_of_night_acts_only_in_its_controllers_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[harbinger_of_night()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let harbinger = on_battlefield(&engine, P0, harbinger_of_night()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let counters = |engine: &Engine<RegistryLookup>, id| {
        engine
            .state()
            .object(id)
            .unwrap()
            .counters
            .get(CounterKind::M1M1)
    };

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(counters(&engine, harbinger), 1, "P1's upkeep added none");
    assert_eq!(counters(&engine, bears), 1, "P1's upkeep added none");

    reach_their_main_phase(&mut engine, P0);
    assert_eq!(counters(&engine, harbinger), 2);
    assert!(
        on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none(),
        "a 2/2 with two -1/-1 counters is dead"
    );
}
