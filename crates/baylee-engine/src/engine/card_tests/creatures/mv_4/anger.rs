//! `cards/creatures/mv_4/anger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn anger_has_haste_on_battlefield_and_grants_your_creatures_haste_from_graveyard_with_a_mountain() {
    let anger = card_index("eaabd151-2160-4bff-82c0-3fa88659be98");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[anger, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(!keywords(&engine, bear).contains(KeywordSet::HASTE));

    for land in all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    let m0 = all_on_battlefield(&engine, p0, mountain())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: m0 })
        .unwrap();
    let anger_card = in_hand(&engine, p0, anger).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: anger_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let anger_perm = on_battlefield(&engine, p0, anger).expect("Anger entered");
    assert!(keywords(&engine, anger_perm).contains(KeywordSet::HASTE));

    let m1 = all_on_battlefield(&engine, p0, mountain())[1];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: m1 })
        .unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![anger_perm],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, anger).is_some());

    assert!(keywords(&engine, bear).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, their_bear).contains(KeywordSet::HASTE));
}
