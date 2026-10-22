//! `cards/enchantments/mv_3/necromancy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn necromancy() -> CardIndex {
    card_index("699b192c-f3f2-4ac5-9ca9-e35bcf08ad8a")
}

/// `Necromancy` ({2}{B}, Enchantment, `Coverage::Partial`) has flash and
/// returns target creature card from a graveyard to the battlefield under your
/// control when it enters.
#[test]
fn necromancy_returns_target_creature_from_graveyard_under_your_control_on_etb() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[necromancy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    kill(&mut engine, bear);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let dead_bear = in_graveyard(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, necromancy());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, dead_bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, necromancy()).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}
