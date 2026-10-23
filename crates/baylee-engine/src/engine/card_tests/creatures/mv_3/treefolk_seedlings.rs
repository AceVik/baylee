#[allow(clippy::wildcard_imports)]
use super::*;

fn treefolk_seedlings() -> CardIndex {
    card_index("87285539-29c8-4cc7-a22c-4a4b96a61e82")
}

#[test]
fn treefolk_seedlings_toughness_equals_forests_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[treefolk_seedlings(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let seedlings = on_battlefield(&engine, p0, treefolk_seedlings()).expect("seedlings on bf");
    assert_eq!(pt(&engine, seedlings), (2, 3));
}
