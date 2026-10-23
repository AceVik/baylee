#[allow(clippy::wildcard_imports)]
use super::*;

fn phyrexian_processor() -> CardIndex {
    card_index("36c800cb-b1ca-4432-ad3c-4d8b90337f4c")
}

#[test]
fn phyrexian_processor_creates_zombie_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[phyrexian_processor(), swamp(), swamp(), swamp(), swamp()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, phyrexian_processor(), 0);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
}
