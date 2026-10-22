//! `cards/creatures/mv_3/crypt_rats.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn crypt_rats() -> CardIndex {
    card_index("104095ed-55e3-408e-bf70-4fe06bb16d2f")
}

#[test]
fn crypt_rats_activated_ability_deals_x_damage_to_each_creature_and_each_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[crypt_rats()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, crypt_rats());
    pass_until(&mut engine, stack_is_empty);

    let rats = on_battlefield(&engine, p0, crypt_rats()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: rats,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, crypt_rats()).is_some());
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().players[1].life, 18);
}
