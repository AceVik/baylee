//! `cards/creatures/mv_5/dread_wight.rs`, played.
//!
//! Oracle:
//! - "At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains '{4}: Remove a paralyzation counter from this creature.'"

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f9078d79-3250-44e8-aba1-ab52e0ba4425"
fn dread_wight() -> CardIndex {
    card_index("f9078d79-3250-44e8-aba1-ab52e0ba4425")
}

fn paralyzation(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .expect("object exists")
        .counters
        .get(counters::PARALYZATION)
}

/// The {4} ability the creature gained, as offered to its controller now.
fn granted_on(engine: &Engine<RegistryLookup>, id: ObjectId) -> Option<(ObjectId, u32)> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        return None;
    };
    legal
        .abilities
        .iter()
        .copied()
        .find(|(source, index)| *source == id && crate::choice::granted_slot(*index).is_some())
}

#[test]
fn dread_wight_paralyzes_its_blocker_until_the_counter_is_paid_off() {
    let mut engine = lk::game(
        SEED,
        &[dread_wight()],
        &[index::WALL_OF_STONE, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let wight = lk::obj(&engine, P0, dread_wight());
    let wall = lk::obj(&engine, P1, index::WALL_OF_STONE);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    let _ = lk::attack(&mut engine, &[wight]);
    lk::block(&mut engine, &[(wall, wight)]);

    // Past the end of combat step.
    lk::through_combat(&mut engine);
    assert_eq!(
        paralyzation(&engine, wall),
        1,
        "at end of combat, a paralyzation counter on each creature blocking Dread Wight"
    );
    assert!(lk::tapped(&engine, wall), "and tap those creatures");
    assert_eq!(
        paralyzation(&engine, bears),
        0,
        "a creature that was not in Dread Wight's fight gets nothing"
    );

    // P1's untap step: the Wall keeps its counter, so it stays tapped.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        lk::tapped(&engine, wall),
        "doesn't untap during its controller's untap step while it has a paralyzation counter"
    );
    assert!(
        granted_on(&engine, bears).is_none(),
        "only those creatures gain the {{4}} ability"
    );

    // Its controller pays {4} to take the counter off.
    lk::float(&mut engine, P1, &[(ManaColor::Colorless, 4)]);
    let (source, ability_index) =
        granted_on(&engine, wall).expect("the Wall gained \"{4}: Remove a paralyzation counter\"");
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("P1 activates the granted ability");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        paralyzation(&engine, wall),
        0,
        "the granted ability removes a paralyzation counter from this creature"
    );
    assert!(
        lk::tapped(&engine, wall),
        "removing it untaps nothing by itself"
    );

    // With no counter left, the next untap step untaps it.
    pass_until(&mut engine, |e| e.state().turn.active == P0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !lk::tapped(&engine, wall),
        "the untap lock lasts only as long as the paralyzation counter"
    );
}
