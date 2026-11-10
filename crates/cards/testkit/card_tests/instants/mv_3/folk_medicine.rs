//! `cards/instants/mv_3/folk_medicine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn folk_medicine_gains_life_per_creature_you_control_and_flashes_back_for_white_mana() {
    let folk_medicine = card_index("135eb851-3f2d-45c5-bfb9-49d1e6c98344");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                plains(),
                plains(),
                grizzly_bears(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[folk_medicine])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    for f in all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: f })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, folk_medicine).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, life_before + 2);
    let gy_spell = in_graveyard(&engine, p0, folk_medicine).unwrap();

    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_spell })
        .expect("flashback Folk Medicine for {1}{W}");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, life_before + 4);
    assert_eq!(engine.state().object(gy_spell).unwrap().zone, Zone::Exile);
}
