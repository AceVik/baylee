#[allow(clippy::wildcard_imports)]
use super::*;

fn citanul_centaurs() -> CardIndex {
    card_index("831487cf-a377-4c9b-920d-c5bc68f17f24")
}

#[test]
fn citanul_centaurs_has_shroud_and_echo() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[citanul_centaurs()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, citanul_centaurs());
    pass_until(&mut engine, stack_is_empty);

    let obj = on_battlefield(&engine, p0, citanul_centaurs()).expect("seated");
    assert_eq!(pt(&engine, obj), (6, 3));
    assert!(keywords(&engine, obj).contains(KeywordSet::SHROUD));
}
