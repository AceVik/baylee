//! `cards/creatures/mv_3/yavimaya_enchantress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn yavimaya_enchantress() -> CardIndex {
    card_index("495fc654-7986-4d76-9d5c-bc1484e98de2")
}

fn crusade() -> CardIndex {
    card_index("4692740f-be90-459f-8d90-c4ae71771595")
}

fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

#[test]
fn yavimaya_enchantress_counts_all_enchantments_on_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[yavimaya_enchantress(), crusade(), disenchant()])
        .battlefield(1, &[crusade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Yavimaya Enchantress for {2}{G}: with opponent's Crusade already on
    // the battlefield (which only pumps white creatures, not green), Yavimaya
    // Enchantress gets +1/+1 for that enchantment and enters as a 3/3.
    cast_from_hand(&mut engine, p0, yavimaya_enchantress());
    pass_until(&mut engine, stack_is_empty);
    let enchantress =
        on_battlefield(&engine, p0, yavimaya_enchantress()).expect("Yavimaya Enchantress resolved");
    assert_eq!(
        pt(&engine, enchantress),
        (3, 3),
        "2/2 base + 1/1 for p1's Crusade on the battlefield"
    );

    // Cast a second Crusade on p0's side (still only pumps white creatures, so
    // only its presence as an enchantment buffs the green Enchantress):
    // Yavimaya Enchantress becomes 2/2 + 2/2 = 4/4.
    cast_with_floating(&mut engine, p0, crusade());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, enchantress), (4, 4));

    // Destroy opponent's Crusade: count drops from 2 to 1.
    let opp_crusade = on_battlefield(&engine, p1, crusade()).unwrap();
    cast_with_floating(&mut engine, p0, disenchant());
    aim_at(&mut engine, p0, opp_crusade);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, enchantress),
        (3, 3),
        "2/2 base + 1/1 for the remaining enchantment on the battlefield"
    );
}
