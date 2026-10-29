//! `cards/instants/mv_2/ray_of_revelation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn ray_of_revelation_destroys_target_enchantment_and_flashes_back_for_green() {
    let ray = card_index("4620bfa8-7198-44ae-a162-51a19ca0f555");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), forest()])
        .battlefield(1, &[their_enchantment(), their_enchantment()])
        .hand(0, &[ray])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let enchantments = all_on_battlefield(&engine, p1, their_enchantment());
    assert_eq!(enchantments.len(), 2);
    let (ench1, ench2) = (enchantments[0], enchantments[1]);

    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, ray).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ench1],
                players: vec![],
            },
        )
        .expect("target first enchantment");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(ench1).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(ench2).unwrap().zone,
        Zone::Battlefield
    );
    let gy_ray = in_graveyard(&engine, p0, ray).expect("Ray of Revelation in graveyard");

    let f = all_on_battlefield(&engine, p0, forest())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: f })
        .unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_ray })
        .expect("flashback Ray of Revelation for {G}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ench2],
                players: vec![],
            },
        )
        .expect("target second enchantment");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(ench2).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(gy_ray).unwrap().zone, Zone::Exile);
}
