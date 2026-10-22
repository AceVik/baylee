//! `cards/enchantments/mv_3/downdraft.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AIR_ELEMENTAL, DOWNDRAFT, GRIZZLY_BEARS};

/// `Downdraft` ({2}{G}, Enchantment) removes flying for `{G}` and sacrifices
/// to deal 2 damage to each creature with flying.
#[test]
fn downdraft_removes_flying_and_sacrifice_deals_2_damage_to_fliers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest(), forest()])
        .battlefield(1, &[AIR_ELEMENTAL])
        .hand(0, &[DOWNDRAFT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elemental = on_battlefield(&engine, p1, AIR_ELEMENTAL).unwrap();
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, DOWNDRAFT);
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, DOWNDRAFT, 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, DOWNDRAFT).is_some());
    assert_eq!(engine.state().object(elemental).unwrap().damage, 2);
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
}
