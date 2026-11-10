//! `cards/instants/mv_1/defy_gravity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn defy_gravity_grants_flying_to_target_creature_and_flashes_back_from_graveyard() {
    let defy_gravity = card_index("5a295dc1-7f60-4581-b0f9-61e8b14339bf");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), grizzly_bears(), gray_ogre()])
        .hand(0, &[defy_gravity])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(!keywords(&engine, bear).contains(KeywordSet::FLYING));

    let u0 = all_on_battlefield(&engine, p0, island())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: u0 })
        .unwrap();
    let spell = in_hand(&engine, p0, defy_gravity).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bear).contains(KeywordSet::FLYING));
    let gy_spell = in_graveyard(&engine, p0, defy_gravity).expect("in graveyard after first cast");

    let u1 = all_on_battlefield(&engine, p0, island())[1];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: u1 })
        .unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_spell })
        .expect("flashback from graveyard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, ogre).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().object(gy_spell).unwrap().zone, Zone::Exile);
}
