//! `cards/enchantments/mv_2/suleiman_s_legacy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MAHAMOTI_DJINN, MOUNTAIN, SERENDIB_EFREET};

fn suleiman_s_legacy() -> CardIndex {
    card_index("9888d67d-fcad-427f-8ba7-b6d421b3d88e")
}

/// `Suleiman's Legacy` ({R}{W}, Enchantment, `Coverage::Implemented`) destroys
/// all Djinns and Efreets when it enters and destroys any Djinn or Efreet that
/// enters afterwards.
#[test]
fn suleiman_s_legacy_destroys_djinns_and_efreets_on_etb_and_when_they_enter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[MOUNTAIN, plains(), island(), island(), island()])
        .battlefield(1, &[MAHAMOTI_DJINN, GRIZZLY_BEARS])
        .hand(0, &[suleiman_s_legacy(), SERENDIB_EFREET])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, suleiman_s_legacy());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, MAHAMOTI_DJINN).is_some());
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Battlefield);

    cast_from_hand(&mut engine, p0, SERENDIB_EFREET);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, SERENDIB_EFREET).is_some());
}
