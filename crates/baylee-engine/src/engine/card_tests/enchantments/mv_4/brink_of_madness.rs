#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn brink_of_madness() -> CardIndex {
    card_index("3209ad08-5482-4dd4-a02e-1dbe99ce6b54")
}

#[test]
fn brink_of_madness_sacrifices_itself_and_makes_opponent_discard_hand_when_hellbent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[brink_of_madness()])
        .hand(1, &[swamp(), swamp()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, brink_of_madness());
    pass_until(&mut engine, stack_is_empty);

    let hand_cards: Vec<_> = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    for c in hand_cards {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                c,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    assert!(engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty());
    assert!(!engine.state().zones.list(ZoneLocation::Hand(p1)).is_empty());

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, brink_of_madness()).is_none());
    assert!(in_graveyard(&engine, p0, brink_of_madness()).is_some());
    assert!(engine.state().zones.list(ZoneLocation::Hand(p1)).is_empty());
}

#[test]
fn brink_of_madness_stays_while_its_controller_has_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[brink_of_madness()],
        &[],
        &[gray_ogre()],
        &[gray_ogre()],
    );
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, brink_of_madness()).is_some());
    assert!(in_hand(&engine, p1, gray_ogre()).is_some());
}
