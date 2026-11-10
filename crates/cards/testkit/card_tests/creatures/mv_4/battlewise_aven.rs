//! `cards/creatures/mv_4/battlewise_aven.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn battlewise_aven_gains_plus_one_plus_one_and_first_strike_with_threshold() {
    let aven = card_index("f165e482-5b87-4062-87ac-bbe8303f74c9");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), swamp()])
        .hand(0, &[aven, dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 6);
    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    let card = in_hand(&engine, p0, aven).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let bird = on_battlefield(&engine, p0, aven).expect("Battlewise Aven entered");
    assert!(keywords(&engine, bird).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, bird).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&engine, bird), (2, 2));

    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bird).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&engine, bird), (3, 3));
}
