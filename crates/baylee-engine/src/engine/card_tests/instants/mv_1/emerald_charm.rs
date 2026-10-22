//! `cards/instants/mv_1/emerald_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::WIND_DRAKE;

fn emerald_charm() -> CardIndex {
    card_index("904b3db5-70bc-4d49-9887-f504aff8666f")
}

#[test]
fn emerald_charm_untaps_permanent_or_removes_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), grizzly_bears()])
        .battlefield(1, &[WIND_DRAKE])
        .hand(0, &[emerald_charm(), emerald_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let drake = on_battlefield(&engine, p1, WIND_DRAKE).unwrap();
    engine.state.set_tapped(bear, true);

    tap_all_mana(&mut engine, p0);

    // Mode 0: Untap target permanent.
    cast_with_floating(&mut engine, p0, emerald_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot0 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot0)).unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, bear));

    // Mode 2: Target creature loses flying until end of turn.
    cast_with_floating(&mut engine, p0, emerald_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot2 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(2)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot2)).unwrap();
    aim_at(&mut engine, p0, drake);
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, drake).contains(KeywordSet::FLYING));
}
