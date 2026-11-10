//! `cards/creatures/mv_4/brawn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn brawn_has_trample_and_grants_your_creatures_trample_from_graveyard_with_a_forest() {
    let brawn = card_index("00876e98-d062-4a12-85e6-86a2b20cf867");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                swamp(),
                swamp(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[brawn, index::TERROR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(!keywords(&engine, bear).contains(KeywordSet::TRAMPLE));

    for land in all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    let brawn_card = in_hand(&engine, p0, brawn).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: brawn_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let brawn_perm = on_battlefield(&engine, p0, brawn).expect("Brawn entered");
    assert!(keywords(&engine, brawn_perm).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&engine, bear).contains(KeywordSet::TRAMPLE));

    for land in all_on_battlefield(&engine, p0, swamp()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    let terror = in_hand(&engine, p0, index::TERROR).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: terror })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![brawn_perm],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, brawn).is_some());

    assert!(keywords(&engine, bear).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&engine, their_bear).contains(KeywordSet::TRAMPLE));
}
