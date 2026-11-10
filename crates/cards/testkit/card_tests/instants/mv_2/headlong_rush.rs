#[allow(clippy::wildcard_imports)]
use super::*;

fn headlong_rush() -> CardIndex {
    card_index("cc3abacc-6643-49c5-8df8-5b56ab20fba5")
}

#[test]
fn headlong_rush_grants_first_strike_to_attacking_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[headlong_rush()])
        .battlefield(0, &[mountain(), mountain(), grizzly_bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    cast_from_hand(&mut engine, p0, headlong_rush());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, headlong_rush()).is_some());
    assert!(keywords(&engine, bear).contains(KeywordSet::FIRST_STRIKE));
    let idle = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(!keywords(&engine, idle).contains(KeywordSet::FIRST_STRIKE));
    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, bear).contains(KeywordSet::FIRST_STRIKE));
}
