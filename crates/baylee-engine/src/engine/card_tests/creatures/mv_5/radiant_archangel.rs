//! `cards/creatures/mv_5/radiant_archangel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn radiant_archangel() -> CardIndex {
    card_index("859cbe11-e1e6-4309-94da-8725db1b3fc2")
}

fn sustainer_of_the_realm() -> CardIndex {
    card_index("2c99d52d-f909-4494-aeee-937ae779fd4a")
}

#[test]
fn radiant_archangel_scales_with_other_flyers() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                sustainer_of_the_realm(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[sustainer_of_the_realm(), gray_ogre()])
        .hand(0, &[radiant_archangel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, radiant_archangel());
    pass_until(&mut engine, stack_is_empty);

    let radiant = on_battlefield(&engine, p0, radiant_archangel()).unwrap();
    assert_eq!(
        pt(&engine, radiant),
        (5, 5),
        "other flyers on both sides, never itself or a ground creature"
    );
    let p1 = PlayerId::new(1);
    let enemy_flyer = on_battlefield(&engine, p1, sustainer_of_the_realm()).unwrap();
    kill(&mut engine, enemy_flyer);
    assert_eq!(
        pt(&engine, radiant),
        (4, 4),
        "counts update when a flyer leaves"
    );
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let ground = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let offer = crate::engine::card_tests::legends_kit::attack(&mut engine, &[radiant]);
    assert!(!crate::engine::card_tests::legends_kit::may_block(
        &offer, ground, radiant
    ));
    assert!(
        !is_tapped(&engine, radiant),
        "vigilance applies when attacking"
    );
}
