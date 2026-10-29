//! `cards/sorceries/mv_4/swelter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn swelter_deals_two_damage_to_each_of_two_target_creatures() {
    let swelter = card_index("e1169108-1bc8-42ca-a056-a9358fba10cb");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[llanowar_elves(), gray_ogre(), aven_flock()])
        .hand(0, &[swelter])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("Llanowar Elves");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("Gray Ogre");

    for m in all_on_battlefield(&engine, p0, mountain()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, swelter).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves, ogre],
                players: vec![],
            },
        )
        .expect("target two creatures");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p1, aven_flock()).is_some());
}
