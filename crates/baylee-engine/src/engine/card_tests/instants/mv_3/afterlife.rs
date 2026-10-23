//! `cards/instants/mv_3/afterlife.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AFTERLIFE, GRIZZLY_BEARS};

#[test]
fn afterlife_destroys_creature_and_creates_spirit_for_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[AFTERLIFE])
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, AFTERLIFE);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);
    let spirits = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p1
                && obj.token.is_some()
                && obj.characteristics().keywords.contains(KeywordSet::FLYING)
        })
        .count();
    assert_eq!(spirits, 1);
}
