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

#[test]
fn midsummer_revel_uses_sacrificed_sources_last_counter_count_including_zero() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for n in [0, 2] {
        let mut e = lk::game_with(SEED, &[midsummer_revel()], &[], &[], &[], |e| {
            lk::yes_no(e, false);
        });
        let revel = lk::obj(&e, p0, midsummer_revel());
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(revel)
            .unwrap()
            .counters
            .add(baylee_cards_dsl::counters::VERSE, n);
        lk::float(&mut e, p0, &[(ManaColor::Green, 1)]);
        lk::use_ability(&mut e, p0, midsummer_revel(), 1, &[], &[]);
        let tokens = tokens_of(&e, p0);
        assert_eq!(tokens.len(), usize::from(n));
        for token in tokens {
            assert_eq!(pt(&e, token), (3, 3));
        }
    }
}
