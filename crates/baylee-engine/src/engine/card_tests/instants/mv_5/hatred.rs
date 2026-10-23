//! `cards/instants/mv_5/hatred.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn hatred() -> CardIndex {
    card_index("40601061-1b25-4db8-8b99-33324ce945cf")
}

#[test]
fn hatred_pays_x_life_and_pumps_target_creature_by_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(
            0,
            &[gray_ogre(), swamp(), swamp(), swamp(), swamp(), swamp()],
        )
        .hand(0, &[hatred()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    tap_all_mana(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let card = in_hand(&engine, p0, hatred()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(4)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 16);
    assert_eq!(pt(&engine, bears), (6, 2));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, bears), (2, 2));
}
