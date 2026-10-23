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
            ],
        )
        .battlefield(1, &[sustainer_of_the_realm()])
        .hand(0, &[radiant_archangel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, radiant_archangel());
    pass_until(&mut engine, stack_is_empty);

    let radiant = on_battlefield(&engine, p0, radiant_archangel()).unwrap();
    assert_eq!(pt(&engine, radiant), (5, 5));
}
