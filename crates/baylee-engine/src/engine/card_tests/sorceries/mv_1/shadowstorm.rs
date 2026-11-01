//! `cards/sorceries/mv_1/shadowstorm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DAUTHI_HORROR, GRIZZLY_BEARS};

fn shadowstorm() -> CardIndex {
    card_index("6e66e983-6349-428b-b511-8738c03ef8fc")
}

/// `Shadowstorm` ({R}, Sorcery, `Coverage::Implemented`):
/// `Shadowstorm deals 2 damage to each creature with shadow.`
#[test]
fn shadowstorm_deals_two_damage_to_each_creature_with_shadow_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), GRIZZLY_BEARS])
        .battlefield(1, &[DAUTHI_HORROR, GRIZZLY_BEARS])
        .hand(0, &[shadowstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, shadowstorm());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, DAUTHI_HORROR).is_none());
    assert!(in_graveyard(&engine, p1, DAUTHI_HORROR).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
}
