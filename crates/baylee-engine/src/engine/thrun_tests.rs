//! The two September 28 reports about Thrun dying during its own turn.
use super::testkit::*;
use super::*;
use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::CardIndex;

fn thrun() -> CardIndex {
    card_index("789b7af5-ac15-40b6-b5b7-f3fcdcfb52e1")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn downfall() -> CardIndex {
    card_index("03df6a57-37c9-46d3-83b3-4a6240100714")
}

#[test]
fn thrun_survives_destruction_only_during_its_controllers_turn() {
    let me = PlayerId::new(0);
    for active in [me, PlayerId::new(1)] {
        let mut engine = Duel::new(928, basic_forest())
            .battlefield(0, &[thrun(), swamp(), swamp(), swamp()])
            .hand(0, &[downfall()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, active));
        if active != me {
            engine.apply(active, PlayerAction::PassPriority).unwrap();
        }
        let troll = on_battlefield(&engine, me, thrun()).unwrap();
        // Its own controller may target it, regardless of the source color.
        cast_from_hand(&mut engine, me, downfall());
        engine
            .apply(
                me,
                PlayerAction::ChooseObjects {
                    objects: vec![troll],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(on_battlefield(&engine, me, thrun()).is_some(), active == me);
        assert_eq!(in_graveyard(&engine, me, thrun()).is_some(), active != me);
    }
}

#[test]
fn thrun_indestructibility_expires_and_returns_at_turn_boundaries() {
    let me = PlayerId::new(0);
    let mut engine = Duel::new(929, basic_forest())
        .battlefield(0, &[thrun()])
        .start();
    keep_mulligans(&mut engine);
    let troll = on_battlefield(&engine, me, thrun()).unwrap();
    for active in [me, PlayerId::new(1), me] {
        assert!(walk_to_own_main(&mut engine, active));
        assert_eq!(
            engine.state().object(troll).unwrap().characteristics().keywords.contains(KeywordSet::INDESTRUCTIBLE),
            active == me
        );
        // Advance out of this main phase before looking for the next one.
        engine.apply(active, PlayerAction::PassPriority).unwrap();
    }
}
