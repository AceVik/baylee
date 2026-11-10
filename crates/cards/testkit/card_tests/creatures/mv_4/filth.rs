//! `cards/creatures/mv_4/filth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn filth_has_swampwalk_and_grants_your_creatures_swampwalk_from_graveyard_with_a_swamp() {
    let filth = card_index("411484ac-a87d-49e8-a7ef-0cd055a872b6");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[filth, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(!keywords(&engine, bear).contains(KeywordSet::SWAMPWALK));

    for land in all_on_battlefield(&engine, p0, swamp()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    let filth_card = in_hand(&engine, p0, filth).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: filth_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let filth_perm = on_battlefield(&engine, p0, filth).expect("Filth entered");
    assert!(keywords(&engine, filth_perm).contains(KeywordSet::SWAMPWALK));
    assert!(!keywords(&engine, bear).contains(KeywordSet::SWAMPWALK));

    let m0 = all_on_battlefield(&engine, p0, mountain())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: m0 })
        .unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![filth_perm],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, filth).is_some());

    assert!(keywords(&engine, bear).contains(KeywordSet::SWAMPWALK));
    assert!(!keywords(&engine, their_bear).contains(KeywordSet::SWAMPWALK));
}
