#[allow(clippy::wildcard_imports)]
use super::*;

fn contamination() -> CardIndex {
    card_index("4013b4c2-c9ed-4d14-90f0-97214ec1fded")
}

#[test]
fn contamination_resolves_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[contamination()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, contamination());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, contamination()).is_some());
}
