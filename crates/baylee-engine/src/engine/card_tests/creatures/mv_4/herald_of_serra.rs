#[allow(clippy::wildcard_imports)]
use super::*;

fn herald_of_serra() -> CardIndex {
    card_index("bc42f32d-e4e1-4174-904c-6058e487f963")
}

#[test]
fn herald_of_serra_enters_with_flying_and_vigilance() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[herald_of_serra()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, herald_of_serra());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, herald_of_serra()).expect("seated");
    assert_eq!(pt(&engine, perm), (3, 4));
}
