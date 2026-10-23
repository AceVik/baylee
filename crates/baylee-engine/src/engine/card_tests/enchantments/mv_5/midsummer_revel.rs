#[allow(clippy::wildcard_imports)]
use super::*;

fn midsummer_revel() -> CardIndex {
    card_index("d3579282-5045-4b37-9998-9a282e9932b9")
}

#[test]
fn midsummer_revel_adds_verse_counter_and_sacrifices_for_beast_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[midsummer_revel(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, midsummer_revel(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, midsummer_revel()).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .len(),
        2
    );
}
