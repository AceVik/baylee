//! `cards/sorceries/mv_4/venomous_vines.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn venomous_vines_destroys_target_enchanted_permanent() {
    let vines = card_index("6a3052a1-7a0b-4364-883d-cd2abca39c06");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[llanowar_elves(), gray_ogre()])
        .hand(0, &[rancor(), vines])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("Llanowar Elves");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("Gray Ogre");

    // Enchant Llanowar Elves with Rancor ({G}).
    let forests = all_on_battlefield(&engine, p0, forest());
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forests[0] })
        .unwrap();
    let aura = in_hand(&engine, p0, rancor()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: aura })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast Venomous Vines: only enchanted Llanowar Elves is a legal target, not unenchanted Gray Ogre.
    for &f in &forests[1..] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: f })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, vines).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&elves));
    assert!(!options.contains(&ogre));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
}
