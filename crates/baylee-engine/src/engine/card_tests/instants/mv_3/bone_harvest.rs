//! `cards/instants/mv_3/bone_harvest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn bone_harvest() -> CardIndex {
    card_index("0cc38d20-5ae8-413a-9f26-ff13cd92f24c")
}

#[test]
fn bone_harvest_puts_creature_card_on_top_of_library_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[bone_harvest(), GRIZZLY_BEARS])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = in_hand(&engine, p0, GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, bone_harvest());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Library);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().object(bear).unwrap().zone,
        Zone::Hand,
        "Bone Harvest should draw the card placed on top of the library at the next turn's upkeep"
    );
}

#[test]
fn bone_harvest_offers_only_creature_cards_from_your_own_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .hand(0, &[bone_harvest(), swamp()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // p0's graveyard: a Swamp and a Grizzly Bears; p1's: a Grizzly Bears.
    let land = in_hand(&engine, p0, swamp()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            land,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    engine.refresh_offer();
    let mine = in_graveyard(&engine, p0, GRIZZLY_BEARS).unwrap();
    let theirs = in_graveyard(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, bone_harvest());
    let options = aim_at(&mut engine, p0, mine);
    assert!(options.contains(&mine), "your creature card");
    assert!(
        !options.contains(&land),
        "a land card is not a creature card"
    );
    assert!(!options.contains(&theirs), "not the opponent's graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(mine).unwrap().zone, Zone::Library);
    assert_eq!(engine.state().object(theirs).unwrap().zone, Zone::Graveyard);
}
