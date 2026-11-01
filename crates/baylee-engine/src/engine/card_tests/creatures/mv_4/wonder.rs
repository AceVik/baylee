//! `cards/creatures/mv_4/wonder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn wonder_has_flying_and_grants_your_creatures_flying_from_graveyard_with_an_island() {
    let wonder = card_index("232284f7-c623-4895-9ab9-8b1a39926830");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[wonder, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for u in all_on_battlefield(&engine, p0, island()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: u })
            .unwrap();
    }
    let card = in_hand(&engine, p0, wonder).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let inc = on_battlefield(&engine, p0, wonder).expect("Wonder on battlefield");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears");

    assert!(keywords(&engine, inc).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, bears).contains(KeywordSet::FLYING));

    let m = all_on_battlefield(&engine, p0, mountain())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: m })
        .unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![inc],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, wonder).is_some());
    assert!(keywords(&engine, bears).contains(KeywordSet::FLYING));
}
