//! `cards/sorceries/mv_4/ashen_powder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn ashen_powder() -> CardIndex {
    card_index("d906965a-0f33-4d75-8abe-66f2cf4817b8")
}

#[test]
fn ashen_powder_reanimates_opponent_creature_under_your_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[ashen_powder()])
        .hand(1, &[GRIZZLY_BEARS])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = in_hand(&engine, p1, GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, ashen_powder());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    let bear_obj = engine.state().object(bear).unwrap();
    assert_eq!(bear_obj.zone, Zone::Battlefield);
    assert_eq!(bear_obj.controller, p0);
}

#[test]
fn ashen_powder_offers_only_creature_cards_from_an_opponents_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .hand(0, &[ashen_powder()])
        .hand(1, &[swamp()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // p0's graveyard: a Grizzly Bears; p1's: a Grizzly Bears and a Swamp.
    let land = in_hand(&engine, p1, swamp()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            land,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    engine.refresh_offer();
    let own = in_graveyard(&engine, p0, GRIZZLY_BEARS).unwrap();
    let theirs = in_graveyard(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, ashen_powder());
    let options = aim_at(&mut engine, p0, theirs);
    assert!(options.contains(&theirs), "an opponent's creature card");
    assert!(
        !options.contains(&own),
        "not a card from your own graveyard"
    );
    assert!(!options.contains(&land), "not a land card");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(theirs).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(theirs).unwrap().controller, p0);
    assert_eq!(engine.state().object(own).unwrap().zone, Zone::Graveyard);
}
