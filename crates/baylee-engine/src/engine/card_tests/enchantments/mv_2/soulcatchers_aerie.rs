//! `cards/enchantments/mv_2/soulcatchers_aerie.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn soulcatchers_aerie_gains_feather_counter_when_your_bird_dies_and_pumps_all_birds() {
    let aerie = card_index("69565f6e-2e23-4cd8-bd21-1283e06729a3");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                mountain(),
                aven_flock(),
                aven_flock(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[aerie, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    let card = in_hand(&engine, p0, aerie).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aerie_id = on_battlefield(&engine, p0, aerie).expect("Soulcatchers' Aerie on battlefield");
    let birds = all_on_battlefield(&engine, p0, aven_flock());
    assert_eq!(birds.len(), 2);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves");

    assert_eq!(pt(&engine, birds[0]), (2, 3));
    assert_eq!(pt(&engine, birds[1]), (2, 3));
    assert_eq!(pt(&engine, elves), (1, 1));

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
                objects: vec![birds[0]],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(aerie_id)
            .unwrap()
            .counters
            .get(counters::FEATHER),
        1
    );
    assert_eq!(pt(&engine, birds[1]), (3, 4));
    assert_eq!(pt(&engine, elves), (1, 1));
}
