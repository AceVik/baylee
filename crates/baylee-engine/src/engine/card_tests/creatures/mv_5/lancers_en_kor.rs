//! `cards/creatures/mv_5/lancers_en_kor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn lancers_en_kor() -> CardIndex {
    card_index("ef49dc78-9fd9-4cf8-af10-5a6b8ef7fc57")
}

/// Lancers en-Kor (`ef49dc78-9fd9-4cf8-af10-5a6b8ef7fc57`) — {3}{W}{W} 3/3 Creature — Kor Rebel Soldier (Partial):
/// "Trample."
#[test]
fn lancers_en_kor_enters_with_trample() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[lancers_en_kor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lancers_en_kor());
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, lancers_en_kor()).unwrap();
    assert_eq!(power_toughness(&engine, id), (3, 3));
    assert!(keywords(&engine, id).contains(KeywordSet::TRAMPLE));
}

#[test]
fn trample_deals_excess_damage_through_a_one_toughness_blocker() {
    use baylee_core::generated::index;
    let mut e = lk::game(
        SEED,
        &[lancers_en_kor()],
        &[index::SAVANNAH_LIONS],
        &[],
        &[],
    );
    let lancers = lk::obj(&e, P0, lancers_en_kor());
    let lions = lk::obj(&e, lk::P1, index::SAVANNAH_LIONS);
    lk::attackers_offered(&mut e);
    lk::declare(&mut e, &[lancers]);
    lk::block(&mut e, &[(lions, lancers)]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
    assert!(in_graveyard(&e, lk::P1, index::SAVANNAH_LIONS).is_some());
}
