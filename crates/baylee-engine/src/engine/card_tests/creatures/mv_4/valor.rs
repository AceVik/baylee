//! `cards/creatures/mv_4/valor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn valor_has_first_strike_and_grants_your_creatures_first_strike_from_graveyard_with_a_plains() {
    let valor = card_index("b2ac84e3-cc3c-49c6-918b-a407ef1ee06c");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[valor, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    let card = in_hand(&engine, p0, valor).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let inc = on_battlefield(&engine, p0, valor).expect("Valor on battlefield");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears");

    assert!(keywords(&engine, inc).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE));

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

    assert!(in_graveyard(&engine, p0, valor).is_some());
    assert!(keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE));
}
