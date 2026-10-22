//! `cards/creatures/mv_3/knight_of_dawn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn knight_of_dawn() -> CardIndex {
    card_index("51262155-f1be-45a8-a21f-94e920a93e55")
}

/// Knight of Dawn (`51262155-f1be-45a8-a21f-94e920a93e55`) — {1}{W}{W} Creature — Human Knight (Partial):
/// "First strike. {W}{W}: This creature gains protection from the color of your choice until end of turn."
#[test]
fn knight_of_dawn_has_first_strike_and_gains_protection_for_two_white() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[knight_of_dawn(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let knight = on_battlefield(&engine, P0, knight_of_dawn()).unwrap();
    assert!(keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&engine, knight).contains(KeywordSet::PROTECTION_BLACK));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, knight_of_dawn(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, knight).contains(KeywordSet::PROTECTION_BLACK));
}
