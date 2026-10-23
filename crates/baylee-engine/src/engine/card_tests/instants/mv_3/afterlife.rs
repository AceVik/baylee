//! `cards/instants/mv_3/afterlife.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DRUDGE_SKELETONS, GRIZZLY_BEARS, SWAMP};

fn afterlife() -> CardIndex {
    card_index("4c13e2b5-961a-4031-84b1-15bd19b94286")
}

#[test]
fn afterlife_destroys_creature_and_creates_spirit_for_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[afterlife()])
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, afterlife());
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
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the caster gets no Spirit"
    );
}

#[test]
fn afterlife_destroys_through_a_regeneration_shield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[afterlife()])
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[DRUDGE_SKELETONS, SWAMP])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let skeletons = on_battlefield(&engine, p1, DRUDGE_SKELETONS).unwrap();
    cast_from_hand(&mut engine, p0, afterlife());
    aim_at(&mut engine, p0, skeletons);
    // In response, p1 regenerates ({B}).
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, DRUDGE_SKELETONS).is_some(),
        "it can't be regenerated"
    );
    assert_eq!(
        tokens_of(&engine, p1).len(),
        1,
        "its controller gets the Spirit"
    );
}
