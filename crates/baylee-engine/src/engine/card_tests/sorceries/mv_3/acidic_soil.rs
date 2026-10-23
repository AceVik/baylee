#[allow(clippy::wildcard_imports)]
use super::*;

fn acidic_soil() -> CardIndex {
    card_index("4ccc1ac5-db90-4fc9-9795-7afaf7466c3e")
}

#[test]
fn acidic_soil_deals_damage_to_each_player_equal_to_lands_they_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[acidic_soil()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[forest(), forest(), forest(), forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, acidic_soil());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 16);
}
