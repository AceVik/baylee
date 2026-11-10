//! `cards/enchantments/auras/mv_2/lashknife.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn lashknife_grants_first_strike_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[card_index("98d790c0-985f-44a4-b247-0951beea7637")])
        .battlefield(0, &[plains(), plains(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    assert!(!keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE));
    cast_from_hand(
        &mut engine,
        p0,
        card_index("98d790c0-985f-44a4-b247-0951beea7637"),
    );
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE));
}
