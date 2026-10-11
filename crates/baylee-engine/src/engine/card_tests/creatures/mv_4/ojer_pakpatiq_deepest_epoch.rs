//! `cards/creatures/mv_4/ojer_pakpatiq_deepest_epoch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The death ability returns this card directly, tapped, with its back face up.
#[test]
fn ojer_pakpatiq_dies_and_comes_back_as_the_land_on_its_other_face() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .battlefield(1, &[ojer_pakpatiq_deepest_epoch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("the god is out");
    assert_eq!(pt(&engine, god), (4, 3), "a 4/3 before anything happens");
    assert!(
        keywords(&engine, god).contains(KeywordSet::FLYING),
        "and it flies"
    );

    cast_from_hand(&mut engine, p0, heroes_downfall());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Downfall asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&god), "a creature is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![god],
                players: vec![],
            },
        )
        .expect("a target the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let temple =
        on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("it came back at once");
    let t = types(&engine, temple);
    assert!(
        t.contains(TypeSet::LAND),
        "\"transformed\" — the back face is Temple of Cyclical Time"
    );
    assert!(
        !t.contains(TypeSet::CREATURE),
        "and the creature stayed on the front"
    );
    assert!(
        in_graveyard(&engine, p1, ojer_pakpatiq_deepest_epoch()).is_none(),
        "the god is on the battlefield and not in a graveyard"
    );

    assert!(
        is_tapped(&engine, temple),
        "the return explicitly enters tapped"
    );
}
