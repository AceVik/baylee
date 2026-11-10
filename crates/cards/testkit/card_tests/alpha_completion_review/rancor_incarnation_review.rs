//! Independent Rancor zone-following review: CR 400.7e and 400.7.

#[allow(clippy::wildcard_imports)] // Shared real-card behavioral vocabulary.
use super::*;
use baylee_core::generated::index;

const OWNER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: holder, .. } if *holder == player),
    );
}

fn setup() -> (Engine<RegistryLookup>, ObjectId, ObjectId, u32) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                llanowar_elves(),
                index::ETERNAL_WITNESS,
                index::PATROL_HOUND,
                forest(),
                plains(),
            ],
        )
        .hand(0, &[index::RANCOR, ephemerate()])
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[index::DISENCHANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let elf = on_battlefield(&engine, OWNER, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, OWNER, index::RANCOR);
    aim(&mut engine, vec![elf], vec![]);
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, OWNER, index::RANCOR).unwrap();
    let battlefield_version = engine.state().object(aura).unwrap().version;
    assert_eq!(pt(&engine, elf), (3, 1));
    assert!(keywords(&engine, elf).contains(KeywordSet::TRAMPLE));
    priority(&mut engine, OTHER);
    cast_from_hand(&mut engine, OTHER, index::DISENCHANT);
    aim(&mut engine, vec![aura], vec![]);
    pass_until(&mut engine, |e| {
        e.state().object(aura).unwrap().zone == Zone::Graveyard
            && matches!(e.pending(), Pending::Priority { .. })
    });
    let trigger = top(&engine);
    assert!(
        engine
            .state()
            .source_referenced_by(engine.state().source_identity(aura).unwrap())
            .contains(&trigger),
        "the waiting return instruction references Rancor's exact graveyard successor"
    );
    assert_eq!(
        engine
            .state()
            .recorded_ability_source(trigger)
            .unwrap()
            .version,
        battlefield_version,
        "the return instruction's graveyard subject must not replace the departed battlefield source"
    );
    assert_eq!(pt(&engine, elf), (1, 1));
    assert!(!keywords(&engine, elf).contains(KeywordSet::TRAMPLE));
    (engine, aura, trigger, battlefield_version)
}

#[test]
fn rancor_review_own_departure_trigger_returns_its_graveyard_successor() {
    let (mut engine, aura, _, battlefield_version) = setup();
    assert!(engine.state().object(aura).unwrap().version > battlefield_version);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_hand(&engine, OWNER, index::RANCOR), Some(aura));
    assert_eq!(in_graveyard(&engine, OWNER, index::RANCOR), None);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn rancor_review_trigger_does_not_follow_a_card_returned_to_graveyard_by_real_discard() {
    let (mut engine, aura, original_trigger, _) = setup();
    let first_graveyard_version = engine.state().object(aura).unwrap().version;
    let witness = on_battlefield(&engine, OWNER, index::ETERNAL_WITNESS).unwrap();
    priority(&mut engine, OWNER);
    cast_from_hand(&mut engine, OWNER, ephemerate());
    aim(&mut engine, vec![witness], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, vec![aura], vec![]);
    pass_until(&mut engine, |e| {
        e.state().object(aura).unwrap().zone == Zone::Hand
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        in_hand(&engine, OWNER, index::RANCOR),
        Some(aura),
        "Eternal Witness really returned the first graveyard incarnation"
    );
    priority(&mut engine, OWNER);
    activate(&mut engine, OWNER, index::PATROL_HOUND, 0);
    engine
        .apply(
            OWNER,
            PlayerAction::ChooseObjects {
                objects: vec![aura],
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(aura).unwrap().zone, Zone::Graveyard);
    assert!(engine.state().object(aura).unwrap().version > first_graveyard_version);
    assert!(
        !engine
            .state()
            .source_referenced_by(engine.state().source_identity(aura).unwrap())
            .contains(&original_trigger),
        "the unrelated second graveyard incarnation is not referenced by the original trigger"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Stack)
            .contains(&original_trigger),
        "Rancor's original trigger has not resolved yet"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        in_graveyard(&engine, OWNER, index::RANCOR),
        Some(aura),
        "the original trigger cannot return the unrelated second graveyard incarnation"
    );
    assert_eq!(in_hand(&engine, OWNER, index::RANCOR), None);
    let hound = on_battlefield(&engine, OWNER, index::PATROL_HOUND).unwrap();
    assert!(
        keywords(&engine, hound).contains(KeywordSet::FIRST_STRIKE),
        "the real discard activation resolved"
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
