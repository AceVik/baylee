//! `cards/instants/mv_3/mogg_salvage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mogg_salvage_destroys_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[card_index("0923ad85-2ea4-4463-8f71-b7d0a162b486")])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[index::FLOWSTONE_ARMOR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let armor = on_battlefield(&engine, p1, index::FLOWSTONE_ARMOR).expect("Flowstone Armor");
    cast_from_hand(
        &mut engine,
        p0,
        card_index("0923ad85-2ea4-4463-8f71-b7d0a162b486"),
    );
    aim_at(&mut engine, p0, armor);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(on_battlefield(&engine, p1, index::FLOWSTONE_ARMOR), None);
}
